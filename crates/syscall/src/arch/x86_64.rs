#![allow(unused)]

macro_rules! syscall {
  ($($name:ident($a:ident, $($b:ident, $($c:ident, $($d:ident, $($e:ident, $($f:ident, $($g:ident, )?)?)?)?)?)?);)+) => {
      $(
          pub unsafe fn $name($a: usize, $(mut $b: usize, $(mut $c: usize, $(mut $d: usize, $(mut $e: usize, $(mut $f: usize, $(mut $g: usize)?)?)?)?)?)?) -> usize {
              let ret: usize;
              unsafe{core::arch::asm!(
                  "int 0x80",
                  in("rax") $a,
                  $(
                      inlateout("rdi") $b,
                      $(
                        inlateout("rsi") $c,
                          $(
                            inlateout("rdx") $d,
                              $(
                                inlateout("r10") $e,
                                  $(
                                    inlateout("r8") $f,
                                      $(
                                        inlateout("r9") $g,
                                      )?
                                  )?
                              )?
                          )?
                      )?
                  )?
                  lateout("rax") ret,
                  options(nostack),
              );
              ret
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
