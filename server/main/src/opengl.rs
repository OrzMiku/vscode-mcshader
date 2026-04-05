use std::ffi::{CStr, CString, c_int};
use std::ptr;
use std::str;

use gl::types::GLenum;

pub struct OpenGlContext {
    _ctx: glutin::Context<glutin::PossiblyCurrent>,
}

impl OpenGlContext {
    pub fn new() -> Self {
        #[cfg(target_os = "windows")]
        use glutin::platform::windows::EventLoopBuilderExtWindows;
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        use glutin::platform::unix::EventLoopBuilderExtUnix;

        #[cfg(any(
            target_os = "windows",
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        let events_loop = {
            let mut builder = glutin::event_loop::EventLoopBuilder::new();
            builder.with_any_thread(true);
            builder.build()
        };

        #[cfg(not(any(
            target_os = "windows",
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        )))]
        let events_loop = glutin::event_loop::EventLoop::new();

        let not_current_context = glutin::ContextBuilder::new()
            .build_headless(&*events_loop, glutin::dpi::PhysicalSize::new(1, 1))
            .unwrap();

        let context = unsafe { not_current_context.make_current().unwrap() };
        gl::load_with(|symbol| context.get_proc_address(symbol));

        Self { _ctx: context }
    }

    pub fn validate_shader(&self, file_type: gl::types::GLenum, source: &str) -> Option<String> {
        unsafe {
            let shader = gl::CreateShader(file_type);
            let c_str_frag = CString::new(source).unwrap();
            gl::ShaderSource(shader, 1, &c_str_frag.as_ptr(), ptr::null());
            gl::CompileShader(shader);

            // Check for shader compilation errors
            let mut success = gl::FALSE as i32;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            let result = if success == gl::TRUE as i32 {
                None
            } else {
                let mut info_len: c_int = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut info_len);
                let mut info = Vec::with_capacity(info_len as usize);
                gl::GetShaderInfoLog(shader, info_len, ptr::null_mut(), info.as_mut_ptr() as *mut gl::types::GLchar);

                // ignore null for str::from_utf8
                let info_len = match info_len {
                    0 => 0,
                    _ => (info_len - 1) as usize,
                };
                info.set_len(info_len);
                Some(String::from_utf8_unchecked(info))
            };
            gl::DeleteShader(shader);
            result
        }
    }

    #[must_use]
    #[inline]
    pub fn vendor(&self) -> &str {
        self.get_str(gl::VENDOR)
    }

    #[must_use]
    #[inline]
    pub fn renderer(&self) -> &str {
        self.get_str(gl::RENDERER)
    }

    #[must_use]
    fn get_str<'a>(&self, gl_enum: GLenum) -> &'a str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(gl::GetString(gl_enum) as *const _).to_bytes()) }
    }
}

unsafe impl Sync for OpenGlContext {}

unsafe impl Send for OpenGlContext {}
