#![feature(asm)]
#![feature(global_asm)]

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

const GPIO_BASE: usize = 0x3F000000 + 0x200000;

const GPIO_FSEL1: *mut u32 = (GPIO_BASE + 0x04) as *mut u32;
const GPIO_SET0: *mut u32 = (GPIO_BASE + 0x1C) as *mut u32;
const GPIO_CLR0: *mut u32 = (GPIO_BASE + 0x28) as *mut u32;

#[inline(never)]
fn spin_sleep_ms(ms: usize) {
    for _ in 0..(ms * 6000) {
        unsafe { asm!("nop" :::: "volatile"); }
    }
}

unsafe fn kmain() -> ! {
    // STEP 1: Set GPIO Pin 16 as output.
    let mut gpio_fsel1_val = GPIO_FSEL1.read_volatile();
    gpio_fsel1_val |= 1<<18;
    GPIO_FSEL1.write_volatile(gpio_fsel1_val);
    // STEP 2: Continuously set and clear GPIO 16.
    loop {
        GPIO_SET0.write_volatile(1<<16);
        spin_sleep_ms(5000);
        GPIO_CLR0.write_volatile(1<<16);
        spin_sleep_ms(5000);
    }
}