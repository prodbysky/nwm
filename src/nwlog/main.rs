use nix::unistd::execvp;
use std::ffi::CString;

fn main() -> nix::Result<()> {
    let cmd = CString::new("tail").unwrap();

    let args = vec![
        CString::new("tail").unwrap(),
        CString::new("-F").unwrap(),
        CString::new("/tmp/nwm.log").unwrap(),
    ];

    execvp(&cmd, &args)?;

    Ok(())
}
