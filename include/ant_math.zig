const std = @import("std");
const builtin = @import("builtin");
const math = std.math;

pub const PI = comptime math.pi;
pub const PI_2 = comptime PI / 2.0;
pub const PI_4 = comptime PI / 4.0;
pub const TAU = comptime 2.0 * PI;
pub const INV_PI = comptime 1.0 / PI;
pub const DEG_TO_RAD = comptime PI / 180.0;
pub const RAD_TO_DEG = comptime 180.0 / PI;

pub const SIN_COEFFS = comptime [_]f64{
	1.0,        // x^1
	-1.0/6.0,   // x^3
	1.0/120.0,  // x^5
	-1.0/5040.0,// x^7
	1.0/362880.0,// x^9
};

pub inline fn abs(comptime T: type, x: T) T {
	return @call(.always_inline, math.abs, .{x});
}

pub inline fn trunc(comptime T: type, x: T) T {
	return @call(.always_inline, math.trunc, .{x});
}

pub inline fn sign(comptime T: type, x: T) T {
	return if (x > 0) 1 else if (x < 0) -1 else 0;
}

pub inline fn clamp(comptime T: type, x: T, min: T, max: T) T {
	return @max(min, @min(max, x));
}

pub inline fn sin_fast(x: f64) f64 {
	var x_folded = x - TAU * trunc(f64, x / TAU);
	if (x_folded > PI) x_folded -= TAU;
	if (x_folded < -PI) x_folded += TAU;

	const x2 = x_folded * x_folded;
	const x3 = x_folded * x2;
	const x5 = x3 * x2;
	const x7 = x5 * x2;
	const x9 = x7 * x2;

	return x_folded * SIN_COEFFS[0] +
		x3 * SIN_COEFFS[1] +
		x5 * SIN_COEFFS[2] +
		x7 * SIN_COEFFS[3] +
		x9 * SIN_COEFFS[4];
}

pub inline fn cos_fast(x: f64) f64 {
	return sin_fast(x + PI_2);
}

pub inline fn tan_fast(x: f64) f64 {
	return sin_fast(x) / cos_fast(x);
}

//x64
pub inline fn rcp_fast(x: f32) f32 {
	return switch (builtin.target.cpu.arch) {
		.x86_64 => asm volatile ("rcpss %[x], %[res]"
			: [res] "=x" (-> f32)
			: [x] "x" (x)
		),
		else => 1.0 / x,
	};
}

