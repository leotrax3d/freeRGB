// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn direct(c:&[Rgb])->[u8;64]{let mut p=[0;64];p[0]=2;for(i,x)in c.iter().take(20).enumerate(){p[1+i*3..4+i*3].copy_from_slice(&[x.r,x.g,x.b]);}p}
