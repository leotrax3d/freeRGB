// SPDX-License-Identifier: GPL-2.0-or-later
pub mod devices_table; pub mod report; pub mod table;
use freergb_core::{Led,LedLayout,Rgb}; use freergb_module_api::sdk::{Hid,Module}; use freergb_module_api::{Capabilities,DeviceKind,ErrorCode,Identification,InterfaceInfo,ModuleError,ModuleInfo,Restore,Zone,MODULE_API};
pub const MODULE_ID:&str="logitech";
#[derive(Default)] pub struct Logitech{active:bool,leds:usize}
fn err(e:impl std::fmt::Display)->ModuleError{ModuleError::new(ErrorCode::Protocol,e.to_string())}
impl Module for Logitech{
 fn info(&self)->ModuleInfo{ModuleInfo{id:MODULE_ID.into(),version:env!("CARGO_PKG_VERSION").into(),family:"LogitechController".into(),api:MODULE_API}}
 fn identify(&self,i:&InterfaceInfo)->Identification{table::identify(i)}
 fn initialize_with_zones(&mut self,i:&InterfaceInfo,z:Option<&[Zone]>,_:&mut dyn Hid)->Result<Capabilities,ModuleError>{let id=table::identify(i);if !id.supported{return Err(err("unsupported Logitech HID device"));}self.leds=z.map(|x|x.iter().map(|q|q.leds).sum()).unwrap_or(20).clamp(1,240);self.active=true;Ok(Capabilities{name:id.model.unwrap_or_else(||"Logitech".into()),kind:DeviceKind::Accessory,layout:LedLayout{width:self.leds as u16,height:1,leds:(0..self.leds).map(|n|Led{name:format!("LED {}",n+1),pos:(n as u16,0)}).collect()},firmware:None,restore:Restore::LastFrame,zones:vec![Zone{name:"Logitech".into(),leds:self.leds}]})}
 fn initialize(&mut self,i:&InterfaceInfo,h:&mut dyn Hid)->Result<Capabilities,ModuleError>{self.initialize_with_zones(i,None,h)}
 fn frame(&mut self,c:&[Rgb],h:&mut dyn Hid)->Result<(),ModuleError>{if !self.active||c.len()!=self.leds{return Err(err("invalid Logitech frame"));}for x in c.chunks(20){h.write(&report::direct(x)).map_err(err)?;}Ok(())}
 fn shutdown(&mut self,_:Option<Rgb>,_:&mut dyn Hid)->Result<(),ModuleError>{self.active=false;Ok(())}
}
#[cfg(test)]mod tests;
