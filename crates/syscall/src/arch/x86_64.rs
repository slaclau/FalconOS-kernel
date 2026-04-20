#![allow(unused)]

use crate::{SyscallError, SyscallOut};

macro_rules! syscall {
  ($($name:ident($a:ident, $($b:ident, $($c:ident, $($d:ident, $($e:ident, $($f:ident, $($g:ident, )?)?)?)?)?)?);)+) => {
      $(
          pub unsafe fn $name($a: usize, $(mut $b: usize, $(mut $c: usize, $(mut $d: usize, $(mut $e: usize, $(mut $f: usize, $(mut $g: usize)?)?)?)?)?)?) -> Result<[usize; 6], SyscallError> {
              let ret: usize;
              let mut words: [usize; 6] = [0; 6];
              unsafe{core::arch::asm!(
                  "int 0x80",
                  in("rax") $a,
                  $(
                      in("rdi") $b,
                      $(
                        in("rsi") $c,
                          $(
                            in("rdx") $d,
                              $(
                                in("r10") $e,
                                  $(
                                    in("r8") $f,
                                      $(
                                        in("r9") $g,
                                      )?
                                  )?
                              )?
                          )?
                      )?
                  )?
                  lateout("rax") ret,
                  lateout("rdi") words[0],
                  lateout("rsi") words[1],
                  lateout("rdx") words[2],
                  lateout("r10") words[3],
                  lateout("r8") words[4],
                  lateout("r9") words[5],
                  options(nostack),
              );
              SyscallOut::new(ret, words).into()
          }}
      )+
  };
}

syscall! {
  syscall0(a,);
  syscall1(a, b,);
  syscall2(a, b, c,);
  syscall3(a, b, c, d,);
  syscall4(a, b, c, d, e,);
  syscall5(a, b, c, d, e, f,);
  syscall6(a, b, c, d, e, f, g,);
}

pub unsafe fn out_syscall5(
    mut a: usize,
    mut b: usize,
    mut c: usize,
    mut d: usize,
    mut e: usize,
    mut f: usize,
) -> (usize, [usize; 5]) {
    unsafe {
        core::arch::asm!(
          "int 0x80",
          inlateout("rax") a,
          inlateout("rdi") b,
          inlateout("rsi") c,
          inlateout("rdx") d,
          inlateout("r10") e,
          inlateout("r8") f,
        );
    }
    (a, [b, c, d, e, f])
}
