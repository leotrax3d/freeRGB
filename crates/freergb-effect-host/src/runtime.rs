//! One interpreter per activated effect, reused across frames. No WASI or host
//! imports: guests cannot access files, devices, the network or capture inputs.
use freergb_core::{Canvas, Rgb};
use freergb_effects::Effect;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wasmi::{
    Config, Engine, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc,
};

pub const MAX_WASM: usize = 512 * 1024;
const FUEL: u64 = 2_000_000;
pub type Fault = Arc<Mutex<Option<String>>>;

pub struct WasmEffect {
    store: Store<StoreLimits>,
    memory: Memory,
    render: TypedFunc<(f64, i32, i32), i32>,
    fault: Fault,
}

impl WasmEffect {
    pub fn load(bytes: &[u8], parameters: &[f64], fault: Fault) -> Result<Self, String> {
        if bytes.len() > MAX_WASM || parameters.len() > 16 {
            return Err("Effect exceeds package or parameter limit".into());
        }
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, bytes).map_err(|e| e.to_string())?;
        if module.imports().next().is_some() {
            return Err(
                "Effect imports are forbidden in API v1 (no WASI or capture access)".into(),
            );
        }
        let limits = StoreLimitsBuilder::new()
            .memory_size(8 * 1024 * 1024)
            .memories(1)
            .tables(1)
            .table_elements(1024)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(&engine, limits);
        store.limiter(|limits| limits);
        store.set_fuel(FUEL).map_err(|e| e.to_string())?;
        let instance = Linker::new(&engine)
            .instantiate(&mut store, &module)
            .map_err(|e| e.to_string())?
            .ensure_no_start(&mut store)
            .map_err(|e| e.to_string())?;
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or("Missing exported memory")?;
        let set = instance
            .get_typed_func::<(i32, f64), ()>(&store, "set_param")
            .map_err(|e| e.to_string())?;
        for (i, value) in parameters.iter().enumerate() {
            set.call(&mut store, (i as i32, *value))
                .map_err(|e| e.to_string())?;
        }
        let render = instance
            .get_typed_func::<(f64, i32, i32), i32>(&store, "render")
            .map_err(|e| e.to_string())?;
        Ok(Self {
            store,
            memory,
            render,
            fault,
        })
    }

    pub fn fault() -> Fault {
        Arc::new(Mutex::new(None))
    }

    fn frame(&mut self, t: Duration, canvas: &mut Canvas) -> Result<(), String> {
        // A shared 128x64 effect surface keeps work independent of LED count.
        let w = canvas.width().min(128);
        let h = canvas.height().min(64);
        self.store.set_fuel(FUEL).map_err(|e| e.to_string())?;
        let ptr = self
            .render
            .call(&mut self.store, (t.as_secs_f64(), w as i32, h as i32))
            .map_err(|e| format!("Effect stopped: {e}"))?;
        let start = usize::try_from(ptr).map_err(|_| "Negative frame pointer")?;
        let end = start.checked_add(w * h * 3).ok_or("Invalid frame length")?;
        let data = self
            .memory
            .data(&self.store)
            .get(start..end)
            .ok_or("Frame outside guest memory")?;
        canvas.paint(|x, y| {
            let i = (((y * h as f32) as usize).min(h - 1) * w
                + ((x * w as f32) as usize).min(w - 1))
                * 3;
            Rgb::new(data[i], data[i + 1], data[i + 2])
        });
        Ok(())
    }
}

impl Effect for WasmEffect {
    fn name(&self) -> &str {
        "Marketplace"
    }
    fn render(&mut self, t: Duration, canvas: &mut Canvas) {
        if self
            .fault
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
        {
            canvas.fill(Rgb::BLACK);
            return;
        }
        if let Err(error) = self.frame(t, canvas) {
            *self.fault.lock().unwrap_or_else(|e| e.into_inner()) = Some(error);
            canvas.fill(Rgb::BLACK);
        }
    }
}
