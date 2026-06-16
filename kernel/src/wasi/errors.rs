//! WASI errno values, mirrored from preview-2's `error-code` enum.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum WasiErr {
    Success         = 0,
    Access          = 1,
    AddrInUse       = 2,
    AddrNotAvail    = 3,
    Again           = 4,
    Already         = 5,
    BadF            = 6,
    Busy            = 7,
    ConnAborted     = 8,
    ConnRefused     = 9,
    ConnReset       = 10,
    Exist           = 11,
    Inval           = 12,
    Io              = 13,
    IsDir           = 14,
    NoEnt           = 15,
    NoMem           = 16,
    NotConn         = 17,
    NotDir          = 18,
    NotSup          = 19,
    Perm            = 20,
    Pipe            = 21,
    TimedOut        = 22,
    HostUnreachable = 23,
}

pub type WasiResult<T> = Result<T, WasiErr>;
