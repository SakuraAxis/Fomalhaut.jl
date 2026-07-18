# Run the native-Rust and pure-Julia servers in separate processes
# The master control unit is responsible for spawning, waiting for ready, benchmarking

# CMD : julia --project=. scripts/benchmark_axis_websocket_vs_julia.jl

using HTTP
using HTTP.WebSockets
using Statistics
using Printf

const NATIVE_PORT = 8080
const JULIA_PORT = 8081
const WARMUP_SEC = 1.0
const MEASURE_SEC = 8.0
const READY_TIMEOUT = 60.0

const SCRIPT_DIR = @__DIR__
const JULIA_BIN = Base.julia_cmd()
const PROJECT = abspath(joinpath(SCRIPT_DIR, ".."))

function spawn_server(script_name::String)
    script_path = joinpath(SCRIPT_DIR, script_name)
    cmd = `$JULIA_BIN --project=$PROJECT $script_path`
    io_out = Pipe()
    proc = run(pipeline(cmd; stdout=io_out, stderr=io_out); wait=false)
    close(io_out.in) # Allow child process to continue writing

    ready = false
    t0 = time()
    ready_ch = Channel{Bool}(1)

    reader = @async begin
        while !eof(io_out)
            line = readline(io_out)
            isempty(line) || println("  [", script_name, "] ", line)
            if occursin("READY", line)
                put!(ready_ch, true)
            end
        end
    end

    while !ready && (time() - t0) < READY_TIMEOUT
        if isready(ready_ch)
            take!(ready_ch)
            ready = true
        end
        !process_running(proc) && error("$script_name exited early — check its output above.")
        sleep(0.05)
    end

    ready || error("$script_name did not become ready within $READY_TIMEOUT s.")
    sleep(0.5) # Server socket binding buffer time

    return proc
end

struct BenchResult
    name::String
    n_frames::Int
    bytes_total::Int
    fps::Float64
    mbps::Float64
    gap_ms::Vector{Float64}
end

function bench_endpoint(name::String, url::String; warmup=WARMUP_SEC, measure=MEASURE_SEC)
    @info "Benchmarking..." name url

    n_frames = 0
    bytes_total = 0
    gaps = Float64[]

    warm_done = false
    t_measure_start = 0.0
    t_prev_frame = 0.0
    t_conn_start = time()

    WebSockets.open(url) do ws
        for msg in ws
            now = time()

            if !warm_done
                if (now - t_conn_start) >= warmup
                    warm_done = true
                    t_measure_start = now
                    t_prev_frame = now
                end
                continue
            end

            if msg isa AbstractVector{UInt8} && !isempty(msg)
                push!(gaps, (now - t_prev_frame) * 1000)
                t_prev_frame = now
                n_frames += 1
                bytes_total += length(msg)
            end

            (now - t_measure_start) >= measure && break
        end
    end

    elapsed = max(time() - t_measure_start, eps())
    fps = n_frames / elapsed
    mbps = (bytes_total / 1_048_576) / elapsed

    return BenchResult(name, n_frames, bytes_total, fps, mbps, gaps)
end

function print_result(r::BenchResult)
    @printf("||||||  %s  ||||||\n", r.name)

    @printf("  Frames received     : %d\n", r.n_frames)
    @printf("  Total payload       : %.2f MB\n", r.bytes_total / 1_048_576)
    @printf("  Throughput          : %.2f fps\n", r.fps)
    @printf("  Bandwidth           : %.2f MB/s\n", r.mbps)
    if !isempty(r.gap_ms)
        @printf("  Inter-frame avg     : %.3f ms\n", mean(r.gap_ms))
        @printf("  Inter-frame p95     : %.3f ms\n", quantile(r.gap_ms, 0.95))
        @printf("  Inter-frame max     : %.3f ms\n", maximum(r.gap_ms))
    end
end

function main()
    @info "Spawning native ( Axis ) server process..."
    proc_native = spawn_server("benchmark_axis_websocket_vs_julia/_bench_server_native.jl")

    @info "Spawning pure-Julia server process..."
    proc_julia = spawn_server("benchmark_axis_websocket_vs_julia/_bench_server_julia.jl")

    results = BenchResult[]

    try
        push!(results, bench_endpoint("Native ( Axis / Rust )", "ws://127.0.0.1:$NATIVE_PORT/live-wave"))
        print_result(results[end])

        push!(results, bench_endpoint("Pure-Julia", "ws://127.0.0.1:$JULIA_PORT/live-wave"))
        print_result(results[end])

        println("||||||  Summary — Native vs Pure-Julia  ( RES=1024, ~4MB/frame )  ||||||")

        native, julia = results[1], results[2]
        @printf("  FPS ratio        : %.2fx\n", native.fps / julia.fps)
        @printf("  Bandwidth ratio  : %.2fx\n", native.mbps / julia.mbps)
        if !isempty(native.gap_ms) && !isempty(julia.gap_ms)
            @printf("  Avg gap ratio    : %.2fx ( julia / native )\n", mean(julia.gap_ms) / mean(native.gap_ms))
        end
    finally
        @info "Shutting down server processes..."
        kill(proc_native)
        kill(proc_julia)
    end
end

main()
