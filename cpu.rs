

// This, like any cpu emulator, is a fucking mess.
//use std::io::File;
extern crate std;
use std::io::println;
//use std::os::args;
use mem::Mem;
struct Reg {
	v : u16
}

fn sign(v : u8) -> i8 {
	// Dunno how to cast to signed :S
	if (v & 0x80) == 0x80 {
		-((!v)+1) as i8
	} else {
		v as i8
	}
}

impl Reg {
	fn set_high(&mut self, v : u8) {
		self.v = (v as u16 << 8) | (self.v & 0xFF) as u16;
	}
	fn set_low(&mut self, v : u8) {
		self.v = (self.v & 0xFF00) as u16 | v as u16
	}
	fn get_high(&self) -> u8 {
		(self.v >> 8) as u8
	}
	fn get_low(&self) -> u8 {
		(self.v & 0xFF) as u8
	}

	fn inc_high(&mut self) -> (u8, u8) {
		let o = self.get_high();
		self.set_high(o+1);
		(o, o+1)
	}
	fn inc_low(&mut self) -> (u8, u8) {
		let o = self.get_low();
		self.set_low(o+1);
		(o, o+1)
	}
	fn inc(&mut self) -> (u16, u16) {
		let o = self.v;
		self.v = o+1;
		(o, o+1)
	}
	fn dec_high(&mut self) -> (u8, u8) {
		let o = self.get_high();
		self.set_high(o-1);
		(o, o-1)
	}
	fn dec_low(&mut self) -> (u8, u8) {
		let o = self.get_low();
		self.set_low(o-1);
		(o, o-1)
	}
	fn dec(&mut self) -> (u16, u16) {
		let o = self.v;
		self.v = o-1;
		(o, o-1)
	}
	fn add_high(&mut self, v : u8) -> (u8, u8) {
		let o = self.get_high();
		self.set_high(o+v);
		(o, o+v)
	}
	fn add(&mut self, v : u16) -> (u16, u16) {
		let o = self.v;
		self.v = o+v;
		(o, o+v)
	}
	fn sub_high(&mut self, v : u8) -> (u8, u8) {
		let o = self.get_high();
		self.set_high(o-v);
		(o, o-v)
	}

	fn to_bytes(&self) -> ~[u8] {
		~[self.get_high(), self.get_low()]
	}
}

struct Regs {
	af : Reg,
	bc : Reg,
	de : Reg,
	hl : Reg,
	sp : Reg,
	pc : Reg
}

impl Regs {
	fn new() -> Regs {
		Regs {
		   af: Reg { v: 0x01B0 },
		   bc: Reg { v: 0x0013 },
		   de: Reg { v: 0x00D8 },
		   hl: Reg { v: 0x014D },
		   sp: Reg { v: 0xFFFE },
		   pc: Reg { v: 0x0100 },
		}
	}
}

pub struct Cpu {
	regs : Regs,
	pub mem : Mem,
	clock : uint,
	screen_mode : int,
	pub drawing : bool,
	interrupts_enabled : bool,
}

impl Cpu {
	pub fn new(rom : ~[u8]) -> Cpu {
		Cpu {
			regs : Regs::new(),
			mem : Mem::new(rom),
			clock : 0,
			screen_mode : 0,
			drawing : false,
			interrupts_enabled : true,
		}
	}
	fn ei(&mut self) {
		self.interrupts_enabled = true
	}
	fn di(&mut self) {
		self.interrupts_enabled = false
	}
	// Tests for carry
	fn ca8(&mut self, (old, new) : (u8, u8)) -> u8 {
		self.set_carry_flag(new < old);
		new
	}
	fn ca16(&mut self, (old, new) : (u16, u16)) -> u16 {
		self.set_carry_flag(new < old);
		new
	}
	// subtraction
	fn cs8(&mut self, (old, new) : (u8, u8)) -> u8 {
		self.set_carry_flag(new > old);
		new
	}
	fn cs16(&mut self, (old, new) : (u16, u16)) -> u16 {
		self.set_carry_flag(new > old);
		new
	}
	// Tests for zero in 8bit registers
	fn z8(&mut self, val : u8) {
		self.set_zero_flag(val == 0);
	}

	fn z16(&mut self, val : u16) {
		self.set_zero_flag(val == 0);
	}

	fn incflags(&mut self, t : (u8, u8)) {
		let r = self.ca8(t);
		self.z8(r);
		self.set_addsub_flag(false);
	}

	fn incflags16(&mut self, t : (u16, u16)) {
		let r = self.ca16(t);
		self.z16(r);
		self.set_addsub_flag(false);
	}

	fn addflags(&mut self, t : (u8, u8)) {
		self.incflags(t);
		self.set_addsub_flag(true);
	}

	fn addflags16(&mut self, t : (u16, u16)) {
		self.incflags16(t);
		self.set_addsub_flag(true);
	}

	fn decflags(&mut self, t : (u8, u8)) {
		let r = self.cs8(t);
		self.z8(r);
		self.set_addsub_flag(false);
	}

	fn decflags16(&mut self, t : (u16, u16)) {
		let r = self.cs16(t);
		self.z16(r);
		self.set_addsub_flag(false);
	}

	fn subflags(&mut self, t : (u8, u8)) {
		self.decflags(t);
		self.set_addsub_flag(true);
	}

	fn push(&mut self, v : u16) {
		self.regs.sp.v -= 2;
		let m : ~[u8] = ~[(v & 0xFF) as u8, (v >> 8) as u8];
		self.mem.write(self.regs.sp.v, m);
	}
	fn pop(&mut self) -> u16 {
		let mut r = self.mem.readbyte(self.regs.sp.v) as u16;
		self.regs.sp.v += 1;
		r |= self.mem.readbyte(self.regs.sp.v) as u16 << 8;
		self.regs.sp.v += 1;
		r
	}
	fn call(&mut self, v : u16) {
		self.push(self.regs.pc.v+1);
		self.regs.pc.v = v-1;
	}
	fn ret(&mut self) {
		self.regs.pc.v = self.pop()-1;
	}
	fn check_carry_flag(&mut self) -> bool {
		self.regs.af.get_low() & (1 << 4) != 0
	}
	fn set_carry_flag(&mut self, v : bool) {
		let n = if v {
			self.regs.af.get_low() | (1 << 4)
		} else {
			self.regs.af.get_low() & !(1 << 4)
		};
		self.regs.af.set_low(n);
	}
	fn check_zero_flag(&mut self) -> bool {
		self.regs.af.get_low() & (1 << 7) != 0
	}
	fn set_zero_flag(&mut self, v : bool) {
		let n = if v {
			self.regs.af.get_low() | (1 << 7)
		} else {
			self.regs.af.get_low() & !(1 << 7)
		};
		self.regs.af.set_low(n);
	}
	fn set_addsub_flag(&mut self, v : bool) {
		let n = if v {
			self.regs.af.get_low() | (1 << 6)
		} else {
			self.regs.af.get_low() & !(1 << 6)
		};
		self.regs.af.set_low(n);
	}
	fn set_hc_flag(&mut self, v : bool) {
		let n = if v {
			self.regs.af.get_low() | (1 << 5)
		} else {
			self.regs.af.get_low() & !(1 << 5)
		};
		self.regs.af.set_low(n);
	}
	fn and(&mut self, v : u8) {
		let a = self.regs.af.get_high() & v;
		self.set_zero_flag(a == 0);
		self.regs.af.set_high(a)
	}
	fn or(&mut self, v : u8) {
		let a = self.regs.af.get_high() | v;
		self.set_zero_flag(a == 0);
		self.regs.af.set_high(a)
	}
	fn xor(&mut self, v : u8) {
		let a = self.regs.af.get_high() ^ v;
		self.set_zero_flag(a == 0);
		self.regs.af.set_high(a)
	}
	fn cp(&mut self, v : u8) {
		let a = self.regs.af.get_high();
		self.set_zero_flag(a == v);
		self.set_carry_flag(a < v);
	}
	fn jr(&mut self, v : u8) {
		self.regs.pc.v = (self.regs.pc.v as i16 + sign(v) as i16) as u16 + 1;
	}
	fn halt(&mut self) {
		fail!("halt, unimplemented")
	}
	fn run_clock(&mut self) {
		self.clock += 4; // TODO: precise cycles
		// self.mem.mem[0xff44] holds the line number
		match self.screen_mode {
			// HBlank
			0 => {
				if self.clock >= 204 {
					self.clock = 0;
					self.mem.mem[0xff44] += 1;
					if self.mem.mem[0xff44] == 143 {
						self.drawing = true;
						self.screen_mode = 1; // Finish, go to VBlank
						self.interrupt(0);
					} else {
						self.screen_mode = 2;
					}
				}
			},
			// VBlank
			1 => {
				if self.clock >= 456 {
					self.clock = 0;
					self.mem.mem[0xff44] += 1;
					if self.mem.mem[0xff44] > 153 {
						self.screen_mode = 2;
						self.mem.mem[0xff44] = 0;
					}
				}
			},
			// OAM Read
			2 => {
				if self.clock >= 80 {
					self.clock = 0;
					self.screen_mode = 3;
				}
			},
			// VRAM Read
			3 => {
				if self.clock >= 172 {
					self.clock = 0;
					self.screen_mode = 0;
				}
			},
			_ => fail!("Wat"),
		}
	}
	pub fn next(&mut self) {
		//if self.regs.pc.v == 0x03C6 {
		//	let mut file = File::create(&Path::new("ram_dump.bin"));
		//	file.write(self.mem.mem);
		//	fail!("quit")
		//}
		let op : u8 = self.mem.readbyte(self.regs.pc.v);
		let n : u8 = self.mem.readbyte(self.regs.pc.v+1);
		let nn : u16 = n as u16 | self.mem.readbyte(self.regs.pc.v+2) as u16 << 8;
		if std::os::args().len() > 2 {
			println("");
			println!("{:04X} {:02X} {:02X} {:02X}\t\tSP: {:04X} AF: {:04X} BC: {:04X} DE: {:04X} HL: {:04X} On Stack: {:04X}",
					 self.regs.pc.v, op, n, nn>>8, self.regs.sp.v,
					 self.regs.af.v, self.regs.bc.v, self.regs.de.v, self.regs.hl.v,
					 self.mem.read16(self.regs.sp.v));
			println!("-6 {:04X} -4 {:04X} -2 {:04X} +0 {:04X} +2 {:04X} +4 {:04X}",
					 self.mem.read16(self.regs.hl.v-6),
					 self.mem.read16(self.regs.hl.v-4),
					 self.mem.read16(self.regs.hl.v-2),
					 self.mem.read16(self.regs.hl.v),
					 self.mem.read16(self.regs.hl.v+2),
					 self.mem.read16(self.regs.hl.v+4));
		}
		match op {
			0x00 => {},
			0x01 => {self.regs.bc.v = nn; self.regs.pc.v += 2},
			0x02 => {self.mem.writebyte(self.regs.bc.v, self.regs.af.get_high())},
			0x03 => {let a = self.regs.bc.inc(); self.incflags16(a)},
			0x04 => {let a = self.regs.bc.inc_high(); self.incflags(a)},
			0x05 => {let a = self.regs.bc.dec_high(); self.decflags(a)},
			0x06 => {self.regs.bc.set_high(n); self.regs.pc.v += 1},
			0x07 => {
				let a = self.regs.af.get_high();
				let b = (a << 1) | (a >> 7);
				self.set_carry_flag(b & 1 == 1);
				self.regs.af.set_high(b)
			},
			0x08 => {self.mem.write(nn, self.regs.sp.to_bytes()); self.regs.pc.v += 2},
			0x09 => {self.regs.hl.v += self.regs.bc.v},
			0x0A => {self.regs.af.set_high(self.mem.readbyte(self.regs.bc.v))},
			0x0B => {self.regs.bc.v -= 1},
			0x0C => {let a = self.regs.bc.inc_low(); self.incflags(a)},
			0x0D => {let a = self.regs.bc.dec_low(); self.decflags(a)},
			0x0E => {self.regs.bc.set_low(n); self.regs.pc.v += 1},
			0x0F => {
				let a = self.regs.af.get_high();
				let b = (a >> 1) | (a << 7);
				self.set_carry_flag(b >> 7 == 1);
				self.regs.af.set_high(b)
			},
			0x10 => {fail!("STOP")},
			0x11 => {self.regs.de.v = nn; self.regs.pc.v += 2},
			0x12 => {self.mem.writebyte(self.regs.de.v, self.regs.af.get_high())},
			0x13 => {let a = self.regs.de.inc(); self.incflags16(a)},
			0x14 => {let a = self.regs.de.inc_high(); self.incflags(a)},
			0x15 => {let a = self.regs.de.dec_high(); self.decflags(a)},
			0x16 => {self.regs.de.set_high(n); self.regs.pc.v += 1},
			0x17 => {
				let a = self.regs.af.get_high();
				self.regs.af.set_high((a << 1) | (a >> 7))
			},
			0x18 => self.jr(n),
			0x19 => {let r = self.regs.hl.add(self.regs.de.v); self.addflags16(r)},
			0x1A => {self.regs.af.set_high(self.mem.readbyte(self.regs.de.v))},
			0x1B => {let f = self.regs.de.dec(); self.decflags16(f)},
			0x1C => {let a = self.regs.de.inc_low(); self.incflags(a)},
			0x1D => {let a = self.regs.de.dec_low(); self.decflags(a)},
			0x1E => {self.regs.de.set_low(n); self.regs.pc.v += 1},
			0x1F => {
				let a = self.regs.af.get_high();
				self.regs.af.set_high((a >> 1) | (a << 7));
			},
			0x20 => if !self.check_zero_flag() {self.jr(n)} else {self.regs.pc.v += 1},
			0x21 => {self.regs.pc.v += 2; self.regs.hl.v = nn},
			0x22 => {
				let addr = self.regs.hl.v;
				self.mem.writebyte(addr, self.regs.af.get_high());
				self.regs.hl.v += 1},
			0x23 => {let a = self.regs.hl.inc(); self.incflags16(a)},
			0x24 => {let a = self.regs.hl.inc_high(); self.incflags(a)},
			0x25 => {let a = self.regs.hl.dec_high(); self.decflags(a)},
			0x26 => {self.regs.hl.set_high(n); self.regs.pc.v += 1},
			0x28 => if self.check_zero_flag() {self.jr(n)} else {self.regs.pc.v += 1},
			0x29 => {let r = self.regs.hl.add(self.regs.hl.v); self.addflags16(r)},
			0x2A => {
				let addr = self.regs.hl.v;
				self.regs.af.set_high(self.mem.readbyte(addr));
				self.regs.hl.v += 1},
			0x2B => {let f = self.regs.hl.dec(); self.decflags16(f)},
			0x2C => {let a = self.regs.hl.inc_low(); self.incflags(a)},
			0x2D => {let a = self.regs.hl.dec_low(); self.decflags(a)},
			0x2E => {self.regs.hl.set_low(n); self.regs.pc.v += 1},
			0x2F => {let a = self.regs.af.get_high(); self.regs.af.set_high(a^0xFF);
				self.set_addsub_flag(true); self.set_hc_flag(true)},

			0x30 => if !self.check_carry_flag() {self.jr(n)} else {self.regs.pc.v += 1},
			0x31 => {self.regs.sp.v = nn; self.regs.pc.v += 2},
			0x32 => {
				let addr = self.regs.hl.v;
				self.mem.writebyte(addr, self.regs.af.get_high());
				self.regs.hl.v -= 1},
			0x33 => {let a = self.regs.sp.inc(); self.incflags16(a)},
			0x34 => {
				let addr = self.regs.hl.v;
				let a = self.mem.readbyte(addr);
				self.mem.writebyte(addr, a+1);
				self.incflags((a, a+1))},
			0x35 => {
				let addr = self.regs.hl.v;
				let a = self.mem.readbyte(addr);
				self.mem.writebyte(addr, a-1);
				self.decflags((a, a-1))},
			0x36 => {let addr = self.regs.hl.v;
				self.mem.writebyte(addr, n);
				self.regs.pc.v += 1},
			0x38 => if self.check_carry_flag() {self.jr(n)} else {self.regs.pc.v += 1},
			0x39 => {self.regs.hl.v += self.regs.sp.v},
			0x3B => {let f = self.regs.sp.dec(); self.decflags16(f)},
			0x3C => {let a = self.regs.af.inc_high(); self.incflags(a)},
			0x3D => {let a = self.regs.af.dec_high(); self.decflags(a)},
			0x3E => {self.regs.af.set_high(n); self.regs.pc.v += 1},
			
			0x40..0xBF => {
				let b = match op & 0x7 {
					0 => self.regs.bc.get_high(),
					1 => self.regs.bc.get_low(),
					2 => self.regs.de.get_high(),
					3 => self.regs.de.get_low(),
					4 => self.regs.hl.get_high(),
					5 => self.regs.hl.get_low(),
					6 => self.mem.readbyte(self.regs.hl.v),
					7 => self.regs.af.get_high(),
					_ => fail!("wat")
				};
				match op {
					0x40..0x47 => self.regs.bc.set_high(b),
					0x48..0x4F => self.regs.bc.set_low(b),
					0x50..0x57 => self.regs.de.set_high(b),
					0x58..0x5F => self.regs.de.set_low(b),
					0x60..0x67 => self.regs.hl.set_high(b),
					0x68..0x6F => self.regs.hl.set_low(b),
					0x70..0x77 => if op == 0x76 {
						self.regs.pc.v -= 1; // Just wait here ok?
					} else {
						self.mem.writebyte(self.regs.hl.v, b)
					},
					0x78..0x7F => self.regs.af.set_high(b),
					0x80..0x87 => {
						let f = self.regs.af.add_high(b);
						self.addflags(f)},
					0x88..0x8F => { //ADC
						let c = self.check_carry_flag();
						let f = self.regs.af.add_high(b+c as u8);
						self.addflags(f)},
					0x90..0x97 => {
						let f = self.regs.af.sub_high(b);
						self.subflags(f)},
					0x98..0x9F => { //SBC
						let c = self.check_carry_flag();
						let f = self.regs.af.sub_high(b-c as u8);
						self.subflags(f)},
					0xA0..0xA7 => {self.and(b)}
					0xA8..0xAF => {self.xor(b)}
					0xB0..0xB7 => {self.or(b)}
					0xB8..0xBF => {self.cp(b)}
					_ => fail!("crash and burn : {:X}", n)
				}
			},

			0xC0 => {if !self.check_zero_flag() {self.ret()}},
			0xC1 => {self.regs.bc.v = self.pop()},
			0xC2 => if !self.check_zero_flag() {self.regs.pc.v = nn} else {self.regs.pc.v += 2},
			0xC3 => {self.regs.pc.v = nn-1},
			0xC4 => {self.regs.pc.v += 2; if !self.check_zero_flag() {self.call(nn)}},
			0xC5 => {self.push(self.regs.bc.v)},
			0xC6 => {
				let f = self.regs.af.add_high(n);
				self.addflags(f);
				self.regs.pc.v += 1},
			0xC7 => self.call(0),
			0xC8 => {if self.check_zero_flag() {self.ret()}},
			0xC9 => {self.ret()},
			0xCA => if self.check_zero_flag() {self.regs.pc.v = nn} else {self.regs.pc.v += 2},
			0xCB => {
				fn f(s : &mut Cpu, n: u8, x: u8) -> u8 {
					if n < 0x8 { // RLC
						let a = (x << 1) | (x >> 7);
						s.set_carry_flag(a & 1 == 1);
						a
					} else if n < 0x10 { // RRC
						let a = (x >> 1) | (x << 7);
						s.set_carry_flag(a >> 7 == 1);
						a
					} else if n < 0x18 { // RL
						(x << 1) | (x >> 7)
					} else if n < 0x20 { // RR
						(x >> 1) | (x << 7)
					} else if n < 0x28 { // SLA
						x << 1
					} else if n < 0x30 { // SRA
						x >> 1
					} else if n < 0x38 { // SWAP
						x << 4 | x >> 4
					} else if n < 0x40 { // SRL
						x >> 1
					} else if n < 0x80 { // BIT
						let b = n >> 3; let c = (x >> b) & 1;
						s.set_zero_flag(c != 1);
						c
					} else if n < 0xC0 { // RES
						let b = ((n >> 3) & 0xF)-1; x & (0xFF ^ (1 << b))
					} else { // SET
						let b = ((n >> 3) & 0xF)-1; x | (1 << b)
					}
				}
				match n & 7 {
					0 => {let x = self.regs.bc.get_high(); let r = f(self, n, x); self.regs.bc.set_high(r)},
					1 => {let x = self.regs.bc.get_low(); let r = f(self, n, x); self.regs.bc.set_low(r)},
					2 => {let x = self.regs.de.get_high(); let r = f(self, n, x); self.regs.de.set_high(r)},
					3 => {let x = self.regs.de.get_low(); let r = f(self, n, x); self.regs.de.set_low(r)},
					4 => {let x = self.regs.hl.get_high(); let r = f(self, n, x); self.regs.hl.set_high(r)},
					5 => {let x = self.regs.hl.get_low(); let r = f(self, n, x); self.regs.hl.set_low(r)},
					6 => {let a = self.regs.hl.v;
							let x = self.mem.readbyte(a);
							let r = f(self, n, x);
							self.mem.writebyte(a, r)},
					7 => {let x = self.regs.af.get_high(); let r = f(self, n, x); self.regs.af.set_high(r)},
					_ => fail!("wat.")
				}
				self.regs.pc.v += 1;
			},
			0xCC => {self.regs.pc.v += 2; if self.check_zero_flag() {self.call(nn)}},
			0xCD => {self.regs.pc.v += 2; self.call(nn)},
			0xCE => {
				let c = self.check_carry_flag() as u8;
				let f = self.regs.af.add_high(n+c);
				self.addflags(f);
				self.regs.pc.v += 1},
			0xCF => self.call(0x08),
			0xD0 => {if !self.check_carry_flag() {self.ret()}},
			0xD1 => {self.regs.de.v = self.pop()},
			0xD2 => if !self.check_carry_flag() {self.regs.pc.v = nn} else {self.regs.pc.v += 2},
			// D3 does not exist
			0xD4 => {self.regs.pc.v += 2; if !self.check_carry_flag() {self.call(nn)}},
			0xD5 => {self.push(self.regs.de.v)},
			0xD6 => {
				let f = self.regs.af.sub_high(n);
				self.subflags(f);
				self.regs.pc.v += 1},
			0xD7 => self.call(0x10),
			0xD8 => {if self.check_carry_flag() {self.ret()}},
			0xD9 => {self.ei(); self.ret()},
			0xDA => if self.check_carry_flag() {self.regs.pc.v = nn} else {self.regs.pc.v += 2},
			// DB does not exist
			0xDC => {self.regs.pc.v += 2; if self.check_carry_flag() {self.call(nn)}},
			// DD does not exist
			0xDE => {
				self.regs.pc.v += 1;
				let c = self.check_carry_flag();
				let f = self.regs.af.sub_high(n-c as u8);
				self.subflags(f)},
			0xDF => self.call(0x18),
			0xE0 => {
				let addr : u16 = 0xFF00 + n as u16;
				self.mem.writebyte(addr, self.regs.af.get_high());
				self.regs.pc.v += 1;
			},
			0xE1 => {self.regs.hl.v = self.pop()},
			0xE2 => {
				let addr : u16 = 0xFF00 + self.regs.bc.get_high() as u16;
				self.mem.writebyte(addr, self.regs.af.get_high());
			},
			// E3 and E4 do not exist
			0xE5 => {self.push(self.regs.hl.v)},
			0xE6 => {self.and(n); self.regs.pc.v += 1},
			0xE7 => self.call(0x20),
			0xE8 => {
				let r = (self.regs.sp.v as i16 + sign(n) as i16) as u16;
				self.regs.sp.v = r;
				self.regs.pc.v += 1;},
			0xE9 => { // Docs says its a jump to (hl), but seems it's jp hl
				self.regs.pc.v = self.regs.hl.v-1},
			0xEA => {
				self.mem.writebyte(nn, self.regs.af.get_high());
				self.regs.pc.v += 2;
			},
			// EB, EC and ED do not exist
			0xEE => {self.xor(n); self.regs.pc.v += 1},
			0xEF => self.call(0x28),
			0xF0 => {
				let addr : u16 = 0xFF00 + n as u16;
				self.regs.af.set_high(self.mem.readbyte(addr));
				self.regs.pc.v += 1;
			},
			0xF1 => {self.regs.af.v = self.pop()},
			0xF2 => {
				let addr : u16 = 0xFF00 + self.regs.bc.get_high() as u16;
				self.regs.af.set_high(self.mem.readbyte(addr));
			},
			0xF3 => {self.di()},
			// F4 does not exist
			0xF5 => {self.push(self.regs.af.v)},
			0xF6 => {self.or(n); self.regs.pc.v += 1},
			0xF7 => self.call(0x30),
			0xF8 => {
				let r = (self.regs.sp.v as i16 + sign(n) as i16) as u16;
				self.regs.hl.v = r;
				self.regs.pc.v += 1;},
			0xF9 => {self.regs.sp.v = self.regs.hl.v},
			0xFA => {
				self.regs.pc.v += 2;
				self.regs.af.set_high(self.mem.readbyte(nn))
			},
			0xFB => self.ei(),
			// FC and FD do not exist
			0xFE => {self.regs.pc.v += 1; self.cp(n)},
			0xFF => self.call(0x38),
			_ => {fail!("Unimplemented OP: {:X}h", op)},
		}

		self.regs.pc.v += 1;
		self.run_clock();
	}
	pub fn interrupt(&mut self, n : u8) {
		if !self.interrupts_enabled {
			return;
		}
		let a = match n {
			0 => 0x40,
			1 => 0x48,
			2 => 0x50,
			3 => 0x58,
			4 => 0x60,
			_ => fail!("Interrupt codes go from 0 to 4"),
		};
		self.call(a);
	}
}
