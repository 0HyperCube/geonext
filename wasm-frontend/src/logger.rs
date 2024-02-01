//! A simple logger implementing the log facade that sends logs to the js console.

use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use wasm_bindgen::prelude::*;

struct SimpleLogger;

#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen(js_namespace = console)]
	fn error(s: &str);
	#[wasm_bindgen(js_namespace = console)]
	fn warn(s: &str);
	#[wasm_bindgen(js_namespace = console)]
	fn info(s: &str);
	#[wasm_bindgen(js_namespace = console)]
	fn debug(s: &str);
	#[wasm_bindgen(js_namespace = console)]
	fn trace(s: &str);
}

impl log::Log for SimpleLogger {
	fn enabled(&self, _metadata: &Metadata) -> bool {
		true
	}

	fn log(&self, record: &Record) {
		if self.enabled(record.metadata()) {
			let value = format!("{}:{}: {}", record.file().unwrap_or("unknown"), record.line().unwrap_or(0), record.args());
			match record.level() {
				Level::Error => error(&value),
				Level::Warn => warn(&value),
				Level::Info => info(&value),
				Level::Debug => debug(&value),
				Level::Trace => trace(&value),
			}
		}
	}

	fn flush(&self) {}
}

static LOGGER: SimpleLogger = SimpleLogger;

/// Initalise the logger. Should be called at the start of the application and only once
pub fn init() -> Result<(), SetLoggerError> {
	log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info))
}
