//! Template project for Flipper Zero.
//! This app prints "Hello, Rust!" to the console then exits.

#![no_main]
#![no_std]

// Required for panic handler
extern crate alloc;
extern crate flipperzero_alloc;
extern crate flipperzero_rt;

use flipperzero::dialogs::{DialogFileBrowserOptions, DialogsApp};
use flipperzero::furi::string::FuriString;
use flipperzero::io::*;
use flipperzero::storage::{File, OpenOptions};
use flipperzero::{print, println};
use flipperzero_rt::{entry, manifest};
use flipperzero_sys as sys;

use alloc::boxed::Box;
use core::ffi::{c_char, c_void, CStr};
use core::ptr::{null_mut, NonNull};
use sys::c_string;
use ufmt::uwrite;

mod tag;
use tag::TagSize;

// Define the FAP Manifest for this application
manifest!(
    name = "Waveshare",
    app_version = 1,
    has_icon = true,
    icon = "waveshare.icon",
);

// Define the entry function
entry!(main);

enum AppView {
    VariableItemList = 0,
    Widget = 1,
}

enum AppEvent {
    SetTagSize(TagSize),
    OpenImage,
    WriteTag,
    WaitForTag,
}

impl AppEvent {
    pub fn to_int(self) -> u32 {
        match self {
            Self::OpenImage => 1, // needs to match location in menu
            Self::WriteTag => 2,  // needs to match location in menu
            Self::WaitForTag => 3,
            Self::SetTagSize(TagSize::TwoNine) => 4,
            Self::SetTagSize(TagSize::FourTwo) => 5,
            Self::SetTagSize(TagSize::SevenFive) => 6,
        }
    }
}

struct App {
    view_dispatcher: NonNull<sys::ViewDispatcher>,
    variable_item_list: NonNull<sys::VariableItemList>,
    widget: NonNull<sys::Widget>,
    tag_size: TagSize,
    tag_size_menu_item: Option<NonNull<sys::VariableItem>>,
    file_path: Option<FuriString>,
    file_menu_item: Option<NonNull<sys::VariableItem>>,
    file: Option<File>,
}

impl App {
    pub fn new() -> Self {
        App {
            view_dispatcher: unsafe { NonNull::new_unchecked(sys::view_dispatcher_alloc()) },
            variable_item_list: unsafe { NonNull::new_unchecked(sys::variable_item_list_alloc()) },
            widget: unsafe { NonNull::new_unchecked(sys::widget_alloc()) },
            tag_size: TagSize::TwoNine,
            tag_size_menu_item: None,
            file_path: None,
            file_menu_item: None,
            file: None,
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            sys::view_dispatcher_free(self.view_dispatcher.as_ptr());
            sys::variable_item_list_free(self.variable_item_list.as_ptr());
            sys::widget_free(self.widget.as_ptr());
        }
    }
}

fn update_widget(widget: *mut sys::Widget, message: *const c_char) {
    unsafe {
        sys::widget_reset(widget);
        sys::widget_add_string_element(
            widget,
            64,
            32,
            sys::Align_AlignCenter,
            sys::Align_AlignCenter,
            sys::Font_FontPrimary,
            message,
        );
    }
}

pub unsafe extern "C" fn item_enter_callback(context: *mut c_void, index: u32) {
    println!("item enter callback index {}", index);
    let app = context as *mut App;
    match index {
        i if i == AppEvent::OpenImage.to_int() => {
            println!("open image selected");
            sys::view_dispatcher_send_custom_event(
                (*app).view_dispatcher.as_ptr(),
                AppEvent::OpenImage.to_int(),
            );
        }
        i if i == AppEvent::WriteTag.to_int() => {
            println!("write tag selected");
            sys::view_dispatcher_send_custom_event(
                (*app).view_dispatcher.as_ptr(),
                AppEvent::WriteTag.to_int(),
            );
        }
        _ => println!("unknown item enter index {}", index),
    }
}

pub unsafe extern "C" fn set_tag_size_callback(item: *mut sys::VariableItem) {
    let index = sys::variable_item_get_current_value_index(item);
    let app = sys::variable_item_get_context(item) as *mut App;
    let view_dispatcher = (*app).view_dispatcher.as_ptr();

    match index {
        i if i == TagSize::TwoNine as u8 => {
            sys::view_dispatcher_send_custom_event(
                view_dispatcher,
                AppEvent::SetTagSize(TagSize::TwoNine).to_int(),
            );
        }
        i if i == TagSize::FourTwo as u8 => {
            sys::view_dispatcher_send_custom_event(
                view_dispatcher,
                AppEvent::SetTagSize(TagSize::FourTwo).to_int(),
            );
        }

        i if i == TagSize::SevenFive as u8 => {
            sys::view_dispatcher_send_custom_event(
                view_dispatcher,
                AppEvent::SetTagSize(TagSize::SevenFive).to_int(),
            );
        }

        _ => println!("unknown menu item index {}", index),
    }
}

fn do_variable_item_list(app: *const App) {
    unsafe {
        let app = app as *mut App;
        let variable_item_list = (*app).variable_item_list.as_ptr();

        sys::variable_item_list_set_enter_callback(
            variable_item_list,
            Some(item_enter_callback),
            app as *mut c_void,
        );

        let item = sys::variable_item_list_add(
            variable_item_list,
            c_string!("Tag Size"),
            3,
            Some(set_tag_size_callback),
            app as *mut c_void,
        );
        (*app).tag_size_menu_item = Some(NonNull::new_unchecked(item));

        sys::variable_item_set_current_value_index(item, (*app).tag_size as u8);
        sys::variable_item_set_current_value_text(item, (*app).tag_size.text());

        let item = sys::variable_item_list_add(
            variable_item_list,
            c_string!("Open Image"),
            1,
            None,
            null_mut(),
        );
        (*app).file_menu_item = Some(NonNull::new_unchecked(item));

        sys::variable_item_set_current_value_text(item, c_string!("None"));

        let _ = sys::variable_item_list_add(
            variable_item_list,
            c_string!("Write Tag"),
            1,
            None,
            null_mut(),
        );
    }
}

pub unsafe extern "C" fn custom_event_callback(context: *mut c_void, event: u32) -> bool {
    println!("custom event callback");
    let app = context as *mut App;
    match event {
        evt if evt == AppEvent::SetTagSize(TagSize::TwoNine).to_int() => {
            println!("2.9 tag size selected");
            (*app).tag_size = TagSize::TwoNine;
            if let Some(item) = (*app).tag_size_menu_item {
                sys::variable_item_set_current_value_text(item.as_ptr(), TagSize::TwoNine.text())
            }
        }
        evt if evt == AppEvent::SetTagSize(TagSize::FourTwo).to_int() => {
            println!("4.2 tag size selected");
            (*app).tag_size = TagSize::FourTwo;
            if let Some(item) = (*app).tag_size_menu_item {
                sys::variable_item_set_current_value_text(item.as_ptr(), TagSize::FourTwo.text())
            }
        }
        evt if evt == AppEvent::SetTagSize(TagSize::SevenFive).to_int() => {
            println!("7.5 tag size selected");
            (*app).tag_size = TagSize::SevenFive;
            if let Some(item) = (*app).tag_size_menu_item {
                sys::variable_item_set_current_value_text(item.as_ptr(), TagSize::SevenFive.text())
            }
        }
        evt if evt == AppEvent::OpenImage.to_int() => {
            println!("open image event received");
            let mut dialogs_app = DialogsApp::open();
            let file_browser_options = DialogFileBrowserOptions::new()
                .set_hide_dot_files(true)
                .set_extension(CStr::from_bytes_until_nul(b"pbm\0").unwrap())
                .set_hide_ext(false);
            (*app).file_path = dialogs_app.show_file_browser(None, Some(&file_browser_options));
            match &(*app).file_path {
                Some(file_path) => {
                    println!("file selected {}", file_path);
                    if let Some(item) = (*app).file_menu_item {
                        sys::variable_item_set_current_value_text(
                            item.as_ptr(),
                            file_path.as_c_str().as_ptr(),
                        );
                    }
                }
                None => {
                    println!("no file selected");
                    if let Some(item) = (*app).file_menu_item {
                        sys::variable_item_set_current_value_text(item.as_ptr(), c_string!("None"));
                    }
                }
            };
        }
        evt if evt == AppEvent::WriteTag.to_int() => {
            println!("write tag event received");
            if let Some(file_path) = &(*app).file_path {
                sys::view_dispatcher_switch_to_view(
                    (*app).view_dispatcher.as_ptr(),
                    AppView::Widget as u32,
                );

                if sys::furi_hal_nfc_is_busy() {
                    println!("nfc is busy");
                    update_widget((*app).widget.as_ptr(), c_string!("Can't start NFC"));
                    return true;
                }

                let Ok(mut file) = OpenOptions::new()
                    .read(true)
                    .open_existing(true)
                    .open(file_path.as_c_str()) else {
                        println!("couldn't open file");
                        update_widget((*app).widget.as_ptr(), c_string!("Can't open file"));
                        return true;
                    };

                let header = (*app).tag_size.header();
                let mut buffer: [u8; 11] = Default::default();
                let Ok(_) = file.read(&mut buffer) else {
                        println!("couldn't read from file");
                        update_widget((*app).widget.as_ptr(), c_string!("Can't read file"));
                        return true;
                };

                if &buffer != header {
                    println!("file header doesn't match");
                    update_widget((*app).widget.as_ptr(), c_string!("Bad file format"));
                    return true;
                }

                (*app).file = Some(file);

                update_widget((*app).widget.as_ptr(), c_string!("waiting for tag"));
                sys::view_dispatcher_send_custom_event(
                    (*app).view_dispatcher.as_ptr(),
                    AppEvent::WaitForTag.to_int(),
                );
            }
        }
        evt if evt == AppEvent::WaitForTag.to_int() => {
            let mut dev_data = sys::FuriHalNfcDevData {
                type_: sys::FuriHalNfcType_FuriHalNfcTypeA,
                interface: sys::FuriHalNfcInterface_FuriHalNfcInterfaceRf,
                uid_len: 0,
                uid: Default::default(),
                cuid: 0,
                atqa: Default::default(),
                sak: 0,
            };

            sys::furi_hal_nfc_exit_sleep();

            let timeout = 300;
            if sys::furi_hal_nfc_detect(
                &mut dev_data as *mut sys::FuriHalNfcDevData,
                timeout as u32,
            ) && &dev_data.uid[0..7] == b"WSDZ10m"
            {
                println!("found tag");
                sys::view_dispatcher_stop((*app).view_dispatcher.as_ptr()); // exit back to main()
                return true;
            }

            sys::furi_hal_nfc_sleep();
            sys::furi_delay_ms(50);
            sys::view_dispatcher_send_custom_event(
                (*app).view_dispatcher.as_ptr(),
                AppEvent::WaitForTag.to_int(),
            ); // run wait for tag event again
        }
        _ => println!("unknown app event {}", event),
    }
    true
}

fn do_view_dispatcher(app: *const App) {
    unsafe {
        let view_dispatcher = (*app).view_dispatcher.as_ptr();
        let variable_item_list = (*app).variable_item_list.as_ptr();
        let widget = (*app).widget.as_ptr();

        sys::view_dispatcher_enable_queue(view_dispatcher);
        sys::view_dispatcher_set_event_callback_context(view_dispatcher, app as *mut c_void);

        pub unsafe extern "C" fn navigation_event_callback(_context: *mut c_void) -> bool {
            println!("navigation event callback");
            false // will cause view dispatcher to stop
        }
        sys::view_dispatcher_set_navigation_event_callback(
            view_dispatcher,
            Some(navigation_event_callback),
        );

        sys::view_dispatcher_set_custom_event_callback(
            view_dispatcher,
            Some(custom_event_callback),
        );

        sys::view_dispatcher_add_view(
            view_dispatcher,
            AppView::VariableItemList as u32,
            sys::variable_item_list_get_view(variable_item_list),
        );
        sys::view_dispatcher_add_view(
            view_dispatcher,
            AppView::Widget as u32,
            sys::widget_get_view(widget),
        );

        let gui = sys::furi_record_open(c_string!("gui")) as *mut sys::Gui;
        sys::view_dispatcher_attach_to_gui(
            view_dispatcher,
            gui,
            sys::ViewDispatcherType_ViewDispatcherTypeFullscreen,
        );
        sys::view_dispatcher_switch_to_view(view_dispatcher, AppView::VariableItemList as u32);

        sys::view_dispatcher_run(view_dispatcher);
    }
}

fn do_write_tag(file: &mut File, widget: *mut sys::Widget, tag_size: TagSize) -> i32 {
    unsafe {
        let timeout = 300;
        let mut tx_rx = Box::new(sys::FuriHalNfcTxRxContext {
            tx_data: [0; 512],
            tx_parity: [0; 64],
            tx_bits: 0,
            rx_data: [0; 512],
            rx_parity: [0; 64],
            rx_bits: 0,
            tx_rx_type: sys::FuriHalNfcTxRxType_FuriHalNfcTxRxTypeDefault,
            nfca_signal: null_mut(),
            sniff_tx: None,
            sniff_rx: None,
            sniff_context: null_mut(),
        });

        update_widget(widget, c_string!("setting up"));

        for cmd in tag_size.setup() {
            tx_rx.tx_data[0..cmd.len()].copy_from_slice(&cmd);
            tx_rx.tx_bits = cmd.len() as u16 * 8;
            let result =
                sys::furi_hal_nfc_tx_rx(&mut *tx_rx as *mut sys::FuriHalNfcTxRxContext, timeout);
            if result == false
                || tx_rx.rx_bits != 16
                || tx_rx.rx_data[0] != 0
                || tx_rx.rx_data[1] != 0
            {
                println!("nfc write cmd failure");
                return -1;
            }
        }

        let mut progress = FuriString::new();
        let (mut buffer, preamble) = tag_size.buffer();
        let loops = tag_size.loops();
        for i in 1..=loops {
            let Ok(_) = file.read(&mut buffer[preamble..]) else { return -1; };
            tx_rx.tx_data[0..preamble].copy_from_slice(&buffer[0..preamble]);
            for (dst, src) in tx_rx.tx_data[preamble..buffer.len()]
                .iter_mut()
                .zip(&buffer[preamble..buffer.len()])
            {
                *dst = !(*src);
            }
            tx_rx.tx_bits = buffer.len() as u16 * 8;
            let result =
                sys::furi_hal_nfc_tx_rx(&mut *tx_rx as *mut sys::FuriHalNfcTxRxContext, timeout);

            if result == false
                || tx_rx.rx_bits != 16
                || tx_rx.rx_data[0] != 0
                || tx_rx.rx_data[1] != 0
            {
                println!("nfc write data failure");
                return -1;
            }

            progress.clear();
            let _ = uwrite!(progress, "chunk {}/{}", i, loops).unwrap();
            update_widget(widget, progress.as_c_str().as_ptr());
        }

        for cmd in [tag_size.power_on(), tag_size.refresh()] {
            tx_rx.tx_data[0..cmd.len()].copy_from_slice(&cmd);
            tx_rx.tx_bits = cmd.len() as u16 * 8;
            let result =
                sys::furi_hal_nfc_tx_rx(&mut *tx_rx as *mut sys::FuriHalNfcTxRxContext, timeout);
            if result == false
                || tx_rx.rx_bits != 16
                || tx_rx.rx_data[0] != 0
                || tx_rx.rx_data[1] != 0
            {
                println!("nfc write cmd failure");
                return -1;
            }
        }

        update_widget(widget, c_string!("finishing"));

        let cmd = tag_size.wait();
        let mut i = 0;
        loop {
            tx_rx.tx_data[0..cmd.len()].copy_from_slice(&cmd);
            tx_rx.tx_bits = cmd.len() as u16 * 8;
            let result =
                sys::furi_hal_nfc_tx_rx(&mut *tx_rx as *mut sys::FuriHalNfcTxRxContext, timeout);
            if result == false {
                println!("nfc write cmd failure");
                return -1;
            }
            if tx_rx.rx_bits == 16 && tx_rx.rx_data[0] == 0xFF && tx_rx.rx_data[1] == 0x00 {
                println!("image saved");
                break;
            }
            sys::furi_delay_ms(100);
            i += 1;
            if i > 50 {
                println!("nfc save data failure");

                return -1;
            }
        }

        update_widget(widget, c_string!("done!"));

        let cmd = tag_size.power_off();
        tx_rx.tx_data[0..cmd.len()].copy_from_slice(&cmd);
        tx_rx.tx_bits = cmd.len() as u16 * 8;
        let result =
            sys::furi_hal_nfc_tx_rx(&mut *tx_rx as *mut sys::FuriHalNfcTxRxContext, timeout);
        if result == false || tx_rx.rx_bits != 16 || tx_rx.rx_data[0] != 0 || tx_rx.rx_data[1] != 0
        {
            println!("nfc write cmd failure");
            return -1;
        }
    }
    0
}

fn main(_args: *mut u8) -> i32 {
    let mut app = Box::new(App::new());

    do_variable_item_list(&*app);

    do_view_dispatcher(&*app);

    if app.file.is_none() {
        return -1;
    }

    let mut file = app.file.take().unwrap();

    let ret = do_write_tag(&mut file, app.widget.as_ptr(), app.tag_size);
    unsafe {
        sys::furi_hal_nfc_sleep();
    }

    ret
}
