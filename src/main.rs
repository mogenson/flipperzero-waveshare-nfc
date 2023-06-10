//! Template project for Flipper Zero.
//! This app prints "Hello, Rust!" to the console then exits.

#![no_main]
#![no_std]

// Required for panic handler
extern crate alloc;
extern crate flipperzero_alloc;
extern crate flipperzero_rt;

use flipperzero::dialogs::{DialogFileBrowserOptions, DialogsApp};
use flipperzero::furi::{string::FuriString, thread};
use flipperzero::println;
use flipperzero_rt::{entry, manifest};
use flipperzero_sys as sys;

use alloc::boxed::Box;
use core::ffi::{c_void, CStr};
use core::ptr::{null_mut, NonNull};
use sys::{c_string, furi_delay_ms};
use ufmt::uwrite;

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

#[derive(Clone, Copy)]
enum TagSize {
    // must match location in menu
    TwoNine = 0,
    FourTwo = 1,
    SevenFive = 2,
}

impl TagSize {
    pub fn text(&self) -> *const core::ffi::c_char {
        match self {
            Self::TwoNine => c_string!("2.9\""),
            Self::FourTwo => c_string!("4.2\""),
            Self::SevenFive => c_string!("7.5\""),
        }
    }
}

enum AppEvent {
    SetTagSize(TagSize),
    OpenImage,
    WriteTag,
    QuitThread,
}

impl AppEvent {
    pub fn to_int(self) -> u32 {
        match self {
            Self::OpenImage => 1, // needs to match location in menu
            Self::WriteTag => 2,  // needs to match location in menu
            Self::QuitThread => 3,
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
    join_handle: Option<thread::JoinHandle>,
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
            join_handle: None,
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
                .set_extension(CStr::from_bytes_until_nul(b"txt\0").unwrap())
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
            let widget = (*app).widget.as_ptr() as usize; // lol fuck safety
            sys::view_dispatcher_switch_to_view(
                (*app).view_dispatcher.as_ptr(),
                AppView::Widget as u32,
            );

            (*app).join_handle = Some(thread::spawn(move || {
                let mut percent = FuriString::from("  0%");
                let mut i = 0;
                while thread::get_flags() != AppEvent::QuitThread.to_int() {
                    percent.clear();
                    let _ = uwrite!(percent, "{}%", i);
                    i += 1;
                    sys::widget_reset(widget as *mut sys::Widget);
                    sys::widget_add_string_element(
                        widget as *mut sys::Widget,
                        128 / 2,
                        64 / 2,
                        sys::Align_AlignCenter,
                        sys::Align_AlignCenter,
                        sys::Font_FontPrimary,
                        percent.as_c_str().as_ptr(),
                    );
                    furi_delay_ms(500);
                }
                0
            }));
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

fn main(_args: *mut u8) -> i32 {
    let mut app = Box::new(App::new());

    do_variable_item_list(&*app);

    do_view_dispatcher(&*app);

    if app.join_handle.is_some() {
        let join_handle = app.join_handle.take().unwrap();
        if !join_handle.is_finished() {
            join_handle
                .thread()
                .set_flags(AppEvent::QuitThread.to_int());
            join_handle.join();
        }
    }

    0
}
