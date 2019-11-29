use crate::error::ErrorKind;
use nix::unistd::{ForkResult, setgid, Gid, initgroups, setuid, Uid, fork};
use users::get_user_by_name;
use std::ffi::CString;
use std::env::set_current_dir;
use crate::login::authenticate;
use std::process::{Command, exit};
use users::os::unix::UserExt;
use crate::displayservers::DisplayServer;
use crate::askpass::UserInfo;
use nix::sys::wait::{waitpid, WaitPidFlag};

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
    let (user_info, _) = loop {
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
    start_displayserver(&mut displayservers::x::X::new(tty + 1), de, user_info)?;

    Ok(())
}


pub fn start_displayserver(displayserver: &mut dyn DisplayServer, de: &str, user_info: UserInfo) -> Result<(), ErrorKind>{
    let displaysergver_process = displayserver.pre_suid().expect("Couldn't start display server.");

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
                &user,
                de
            ).expect("Couldn't start X");

            println!("Exiting user process");
            // If X closes back to login?

            exit(0);
        }
        Ok(ForkResult::Parent { child }) => {
            // The parent process where we should handle reboot, lock, etc signals

            println!("Waiting for user process to exit");
            let mut flag : WaitPidFlag = WaitPidFlag::WEXITED;
            flag.insert(WaitPidFlag::WSTOPPED);
            let _ = waitpid(child, Some(flag));
            println!("User process exited, restarting authenticator");


//        if let Some(ref mut child) = &mut self.xorg {
//            println!("Terminating X");
//            let _ = kill(Pid::from_raw(child.id() as i32), nix::sys::signal::SIGTERM);
//            let mut terminated = false;
//            for _ in 1..10{
//                sleep(Duration::from_millis(100));
//                if let Ok(Some(_)) = child.try_wait() {
//                    println!("X is dead!");
//                    terminated = true;
//                    break;
//                }
//                println!("X still hasn't died, waiting for forced termination...");
//            }
//            if !terminated {
//                println!("X was terminated with force");
//                let _ = child.kill();
//            }
//        }

            return main();
        }
        Err(_) => return Err(ErrorKind::ForkFailed)
    };
}