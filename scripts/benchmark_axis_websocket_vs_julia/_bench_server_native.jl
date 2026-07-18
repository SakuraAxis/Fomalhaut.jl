import Fomalhaut as FMHUT
import Axis as AX

const RES = 1024
const R = Float32[-3f0 + 6f0 * (i - 1) / (RES - 1) for i in 1:RES]
const OUT_BUFFER = Vector{Float32}(undef, RES * RES)

mutable struct WaveContext
    start_time_sec::Float64
    r::Ptr{Float32}
    res::Int32
    out::Ptr{Float32}
end

@AX.rust_code """
#[repr(C)]
pub struct WaveContext {
    pub start_time_sec: f64,
    pub r: *const f32,
    pub res: i32,
    pub out: *mut f32,
}
"""

@AX.rust_fn function _wave_native_frame(ctx::Ptr{Cvoid}, out_len::Ptr{Csize_t})::Ptr{UInt8}
    """
    let ctx = unsafe { &mut *(ctx as *mut WaveContext) };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();

    if ctx.start_time_sec == 0.0 {
        ctx.start_time_sec = now;
    }
    let t = ((now - ctx.start_time_sec) * 2.0) as f32;

    let res = ctx.res as usize;
    let r = unsafe { std::slice::from_raw_parts(ctx.r, res) };
    let out = unsafe { std::slice::from_raw_parts_mut(ctx.out, res * res) };

    for i in 0..res {
        for j in 0..res {
            out[i * res + j] = (r[i] + t).sin() + (r[j] + t).cos();
        }
    }

    unsafe {
        *out_len = (res * res * 4) as usize;
        ctx.out as *mut u8
    }
    """
end

const _WAVE_CTX = Ref{WaveContext}()
_WAVE_CTX[] = WaveContext(0.0, pointer(R), Int32(RES), pointer(OUT_BUFFER))

ctx_ptr = Base.unsafe_convert(Ptr{Cvoid}, _WAVE_CTX)

axis_generated_dir = abspath(joinpath(@__DIR__, "..", "..", "axis_rs"))
AX.bridge_up(axis_generated_dir)

cb_ptr = AX._axis_rs_symbol(Symbol("_wave_native_frame"))

app = FMHUT.App()
@FMHUT.axis_websocket app "/live-wave" 60.0 cb_ptr ctx_ptr

println("READY") # Signals for detection by the main control unit
flush(stdout)

FMHUT.serve(app; port=8080, fps=60)
