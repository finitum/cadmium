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
use flexi_logger::{Duplicate, Logger, Criterion, Naming, Cleanup, Age};
use log::{error, warn, info, debug};

mod askpass;
pub mod error;
mod login;
mod dbus;
mod displayservers;

fn initiate_logger() {
    Logger::with_str("info, pam=debug")
        .directory("/var/log/cadmium/")
        .rotate(
            Criterion::Age(Age::Day),
            Naming::Timestamps,
            Cleanup::KeepLogAndZipFiles(3, 15)
        )
        .log_to_file()
        .duplicate_to_stderr(Duplicate::Warn)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
}

fn main() -> Result<(), ErrorKind> {
    initiate_logger();
    start()
}

fn start() -> Result<(), ErrorKind>{

    let tty = 2;
    let de = "bspwm";

    // de-hardcode 2
    if chvt::chvt(tty).is_err() {
        error!("Could not change console");
    };

    // Loop assignment _gasp_
    let user_info = loop {
        match authenticate(tty as u32) {
            Ok(i) => break i,
            Err(e) => match e {
                ErrorKind::AuthenticationError => continue,
                _ => {
                    warn!("Couldn't authenticate: ");
                    return Err(e);
                },
            }
        }
    };

//    if !logind_manager.is_connected() {
//        println!("Couldn't start DBus: ");
//        return Err(ErrorKind::DBusError);
//    }
    start_displayserver(&mut displayservers::x::X::new(tty), de, user_info)?;

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
            info!("Logged in as: {}", std::env::var("USER").expect("USER is not set"));
            debug!("Current directory: {}", std::env::var("PWD").expect("PWD is not set"));
            debug!("shell: {:?}", std::env::var("SHELL").expect("no shell"));

            // From User struct
            debug!("user: {:?}", user);
            debug!("user id: {:?}", user.uid());
            debug!("primary group: {:?}", user.primary_group_id());
            debug!("Home directory: {}", &homedir.display());

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

            info!("Exiting user process");
            // If X closes back to login?

            exit(0);
        }
        Ok(ForkResult::Parent { child }) => {
            // The parent process where we should handle reboot, lock, etc signals

            info!("Waiting for user process to exit");
            let mut flag : WaitPidFlag = WaitPidFlag::WEXITED;
            flag.insert(WaitPidFlag::WSTOPPED);
            let _ = waitpid(child, Some(flag));
            info!("User process exited, restarting authenticator");


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

            return start();
        }
        Err(_) => return Err(ErrorKind::ForkFailed)
    };
}