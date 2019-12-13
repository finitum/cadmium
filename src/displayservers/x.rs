use crate::displayservers::{DisplayServer, DisplayServerError};

use std::{env, fmt};
use std::path::Path;
use std::fs::File;
use std::fmt::Debug;
use std::error::Error;
use rand::Rng;
use std::process::{Command, Child};
use xcb::{Connection, ConnError};
use nix::sys::signal::kill;
use nix::unistd::Pid;
use nix::errno::Errno;
use users::User;
use users::os::unix::UserExt;
use std::thread::sleep;
use std::time::Duration;

#[derive(Debug)]
pub enum XError {
    IOError,
    XAuthError,
    NoFreeDisplayError,
    XStartError,
    DEStartError,
    XCBConnectionError,
    NoSHELLError,
    NoDisplayError
}

impl Error for XError {}
impl fmt::Display for XError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <dyn Debug>::fmt(self, f)
    }
}

pub struct X {
    xcb: Option<Connection>,
    tty: i32,
    display: Option<String>,
    xorg: Option<Child>,
}

impl DisplayServer for X {
    fn pre_suid(&mut self) -> Result<(), DisplayServerError> {
        let display = format!(":{}", Self::get_free_display()?);

        // set the DISPLAY environment variable
        env::set_var("DISPLAY", &display);

        println!("Starting xorg process");
        let xorg_process = Command::new("/usr/bin/X")
            .args(&[&display, &format!("vt{}", self.tty)])
            .spawn().map_err(|_| XError::XStartError)?;

        println!("Waiting for xorg to start");
        // Wait for the process to start running
        // TODO: close xcb connection and save it somewhere in the struct(?)
        let xcb  = loop {
            if let Err(e) = kill(Pid::from_raw(xorg_process.id() as i32), None) {
                match e.as_errno() {
                    Some(e) => match e {
                        Errno::ESRCH => {
                            continue;
                        }
                        _ => return Err(DisplayServerError::XError(XError::XCBConnectionError))
                    }
                    None => return Err(DisplayServerError::XError(XError::XCBConnectionError))
                }
            };

            match Connection::connect(Some(&display)) {
                Ok(c) => break c,
                Err(e) => {
                    match e {
                        ConnError::Connection => continue,
                        _ => return Err(DisplayServerError::XError(XError::XCBConnectionError))
                    }
                }
            }
        };

        self.display = Some(display);
        self.xorg = Some(xorg_process);
        self.xcb = Some(xcb.0);
        Ok(())
    }

    fn post_suid(&mut self, user_info: &User, de: &str) -> Result<(), DisplayServerError> {
        let display = self.display.as_ref().ok_or(XError::NoDisplayError)?;
        Self::xauth(display, user_info.home_dir())?;
        println!("Running DE");

        let mut de_process = Command::new(env::var("SHELL").map_err(|_| XError::NoSHELLError)?)
            .arg("-c").arg("--login").arg(format!("$@={}", de)).arg(include_str!("../../res/xsetup.sh")).spawn().map_err(|_| XError::DEStartError)?;

        let _ = de_process.wait(); // wait for the DE to exit

        println!("DE stopped");

        // Closes the xcb connection
        if let Some(c) = self.xcb.take() {
            drop(c);
        }

        println!("XCB connection closed");

        Ok(())
    }
}

impl X {
    pub fn new(tty: i32) -> Self {
        X { xcb: None, tty, display: None, xorg: None}
    }

    fn mcookie() -> String{
        let mut rng = rand::thread_rng();

        let cookie: u128 = rng.gen();
        format!("{:032x}", cookie)
    }

    /// Loops through all displays and finds the first free one.
    fn get_free_display() -> Result<i32, XError>{
        for i in 0..200 {
            if !Path::new(&format!("/tmp/.X{}-lock", i)).exists() {
                return Ok(i);
            }
        }

        Err(XError::NoFreeDisplayError)
    }

    /// Create our auth file (.cdxauth).
    fn xauth(display: &str, home: &Path) -> Result<(), XError> {
        let xauth_path = home.join(".cdxauth");

        // set the XAUTHORITY environment variable
        env::set_var("XAUTHORITY", &xauth_path);

        File::create(xauth_path).map_err(|_| XError::IOError)?;

        // use `xauth` to generate the xauthority file for us
        Command::new("/usr/bin/xauth")
            .args(&["add", display, ".", &Self::mcookie()])
            .output().map_err(|_| XError::XAuthError)?;

        Ok(())
    }
}

//pub fn start_x(tty: u32, home: &Path, de: &str) -> Result<(), XError> {
//
//    println!("{}",  String::from_utf8_lossy(&Command::new("env").output().expect("couldnt execute env").stdout));
//    std::io::stdin()
//        .bytes()
//        .next()
//        .and_then(|result| result.ok());


#[cfg(test)]
mod test {
    use super::X;

    #[test]
    fn test_mcookie_length() {
        assert_eq!(X::mcookie().len(), 32)
    }

    #[test]
    fn test_mcookie_same() {
        assert_ne!(X::mcookie(), X::mcookie());
    }
}
