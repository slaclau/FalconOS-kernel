#![no_std]
#![no_main]

use hal::halt;
use syscall::_log::show;
use syscall::cap::Rights;
use syscall::format_log;
use syscall::ipc::{Endpoint, IpcStatus};

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &core::panic::PanicInfo) -> ! {
    syscall::log("panic");
    format_log!(
        "with {} at {}/{}",
        info.message(),
        info.location().unwrap().file(),
        info.location().unwrap().line()
    );
    loop {
        hal::halt();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let echo_proc = syscall::cap::Cap::<syscall::process::Process>::spawn(echo, 0)
        .expect("Could not spawn proc");

    let send_ep =
        syscall::cap::Cap::<syscall::ipc::Endpoint>::create().expect("could not create endpoint");

    let recv_ep = send_ep
        .derive(Rights::READ)
        .expect("Could not derive recv endpoint");
    let id = recv_ep.r#move(echo_proc).expect("Could not move recv ep");
    format_log!("r ep is {id}");
    let mut i = 0;
    loop {
        i += 1;
        let buf = &mut [0u8; 32];
        show(buf, format_args!("send {i} to echo")).expect("Could not format message");
        let msg = (*buf).into();
        format_log!(
            "send {:?} to echo",
            str::from_utf8(buf).unwrap().trim_matches(char::from(0))
        );
        let status = send_ep.send(msg).expect("could not send");
        format_log!("got {status:?} from sending");
        match status {
            IpcStatus::WouldBlock => syscall::process::r#yield().expect("could not yield"),
            IpcStatus::Ready => {}
        };
        let resp = send_ep.recv().expect("could not receive").1;
        let bytes = resp.data.map(|word| word.to_be_bytes());
        let bytes = bytes.as_flattened();
        let resp_str = str::from_utf8(bytes).unwrap().trim_matches(char::from(0));
        format_log!("got {resp_str:?} from echo");
        assert_eq!(msg, resp);
        if i > 5 {
            break;
        }
    }
    loop {
        halt();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn echo(parent_ep: usize) -> usize {
    let ep = unsafe { syscall::cap::Cap::<Endpoint>::from_handle(parent_ep) };
    loop {
        let (reply_cap, msg) = ep.recv().expect("could not recv");
        format_log!("got {msg:?} from main");
        reply_cap.reply(msg).expect("could not reply");
        format_log!("sent {msg:?} to main");
    }
}
