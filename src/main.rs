use crate::error::ErrorKind;
use nix::unistd::{ForkResult, setgid, Gid, initgroups, setuid, Uid, fork};
use users::get_user_by_name;
use std::ffi::CString;
use std::env::set_current_dir;
use crate::login::authenticate;
use std::path::Path;
use std::process::Command;
use users::os::unix::UserExt;
use std::time::Duration;
use crate::displayservers::DisplayServer;

mod askpass;
pub mod error;
mod login;
mod dbus;
mod displayservers;

fn main() -> Result<(), ErrorKind>{

    let tty = 2;
    let de = "bspwm";

    // de-hardcode 2
    if chvt::chvt(tty).is_err() {
        println!("Could not change console");
    };

    // Loop assignment _gasp_
    let (user_info, logind_manager) = loop {
        match authenticate(tty as u32) {
            Ok(i) => break i,
            Err(e) => match e {
                ErrorKind::AuthenticationError => continue,
                _ => {
                    println!("Couldn't authenticate: ");
                    return Err(e);
                },
            }
        }
    };

//    if !logind_manager.is_connected() {
//        println!("Couldn't start DBus: ");
//        return Err(ErrorKind::DBusError);
//    }
    start_displayserver(&displayservers::x::X::new(), de);

    Ok(())
}


pub fn start_displayserver(displayserver: &dyn DisplayServer, de: &str) {
    match fork() {
        Ok(ForkResult::Child) => {

            // Get some user info
            let user= get_user_by_name(&user_info.username).expect("Couldn't find username");
            let homedir = user.home_dir();

            // Print some debugging info from ENV
            println!("Logged in as: {}", std::env::var("USER").expect("USER is not set"));
            println!("Current directory: {}", std::env::var("PWD").expect("PWD is not set"));
            println!("shell: {:?}", std::env::var("SHELL").expect("no shell"));

            // From User struct
            println!("user: {:?}", user);
            println!("user id: {:?}", user.uid());
            println!("primary group: {:?}", user.primary_group_id());
            println!("Home directory: {}", &homedir.display());

            // Source locale.conf to set LANG appropriately
            Command::new("bash").arg("-c").arg("/etc/locale.conf").output().expect("Couldn't source language");

            displayserver.pre_suid();

            initgroups(
                &CString::new(user_info.username).unwrap(),
                Gid::from_raw(user.primary_group_id())
            ).expect("Could not init groups for your user");

            setgid(Gid::from_raw(user.primary_group_id())).expect("Could not set GID for the process");

            // No Root :(
            setuid(Uid::from_raw(user.uid())).expect("Could not set UID for the process");

            set_current_dir(&homedir).expect("Couldn't cd to home directory");

//            dbus::start_dbus();

            displayserver.post_suid(
                (tty + 1) as u32, // Start X on tty+1 so that we keep logs here
                &user,
                de
            ).expect("Couldn't start X");

            // If X closes back to login?
        }
        Ok(ForkResult::Parent { child }) => {
            // The parent process where we should handle reboot, lock, etc signals
            loop {
                std::thread::sleep(Duration::from_secs(1)) // So that the loop doesn't get optimized away
            }
        }
        Err(_) => return Err(ErrorKind::ForkFailed)
    }
}