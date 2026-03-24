pub fn halt() {
    unsafe {
        core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

pub mod interrupts {
    pub fn enable(enable: bool) {
        unsafe {
            if enable {
                core::arch::asm!("sti", options(preserves_flags, nostack));
            } else {
                core::arch::asm!("cli", options(preserves_flags, nostack));
            }
        }
    }

    pub fn are_enabled() -> bool {
        let r: u64;

        unsafe {
            core::arch::asm!("pushfq; pop {}", out(reg) r, options(nomem, preserves_flags));
        };

        r & 0x0200 != 0
    }

    pub fn without_interrupts<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let enabled = are_enabled();

        if enabled {
            enable(false);
        }

        let ret = f();

        if enabled {
            enable(true);
        }

        ret
    }
}
