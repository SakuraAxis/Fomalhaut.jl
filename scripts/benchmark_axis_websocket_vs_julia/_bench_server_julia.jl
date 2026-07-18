import Fomalhaut as FMHUT

const RES = 1024
const BUFFER = zeros(Float32, RES, RES)
const R = range(-3f0, 3f0, length=RES)

function wave_stream(ctx)
    t = Float32(ctx.time * 2.0)
    BUFFER .= sin.(R .+ t) .+ cos.(R' .+ t)
    return vec(BUFFER)
end

app = FMHUT.App()
@FMHUT.websocket app "/live-wave" wave_stream

println("READY")
flush(stdout)

FMHUT.serve(app; port=8081, fps=60)
