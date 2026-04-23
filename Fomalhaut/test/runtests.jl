using Test
using Fomalhaut

function free_ffi_buffer(ptr::Ptr{UInt8})
    ptr == C_NULL && return
    ccall((:fmh_free, Fomalhaut._load_rust_lib()), Cvoid, (Ptr{UInt8},), ptr)
end

@testset "App route registration" begin
    app = App()

    @post app "/infer" req -> (copy(req.body), "application/octet-stream")
    @options app "/infer" req -> (UInt8[], "text/plain", 204)
    @websocket app "/stream" ctx -> UInt8[0x01, 0x02]

    @test haskey(app.http_routes, ("POST", "/infer"))
    @test haskey(app.http_routes, ("OPTIONS", "/infer"))
    @test haskey(app.ws_routes, "/stream")
end

@testset "HTTP trampoline" begin
    app = App()
    Fomalhaut._active_app[] = app
    @post app "/infer" req -> (vcat(req.body, UInt8[0xFF]), "application/octet-stream")
    @options app "/infer" req -> (UInt8[], "text/plain", 204)

    post_method = UInt8['P', 'O', 'S', 'T']
    post_path = Vector{UInt8}(codeunits("/infer"))
    post_query = Vector{UInt8}(codeunits("mode=test"))
    post_headers = Vector{UInt8}(codeunits("content-type: application/octet-stream\r\nx-test: 1\r\n"))
    post_body = UInt8[0x10, 0x20]
    post_response = Ref(Fomalhaut.FFIHttpResponse(Ptr{UInt8}(C_NULL), 0, Ptr{UInt8}(C_NULL), 0, 0))

    post_status = Fomalhaut._http_request_trampoline(
        C_NULL,
        pointer(post_method),
        Csize_t(length(post_method)),
        pointer(post_path),
        Csize_t(length(post_path)),
        pointer(post_query),
        Csize_t(length(post_query)),
        pointer(post_headers),
        Csize_t(length(post_headers)),
        pointer(post_body),
        Csize_t(length(post_body)),
        Base.unsafe_convert(Ptr{Fomalhaut.FFIHttpResponse}, post_response),
    )

    @test post_status == 0
    post_stored = post_response[]
    post_out_body = copy(unsafe_wrap(Vector{UInt8}, post_stored.body_ptr, Int(post_stored.body_len)))
    post_out_content_type = String(copy(unsafe_wrap(Vector{UInt8}, post_stored.content_type_ptr, Int(post_stored.content_type_len))))
    free_ffi_buffer(post_stored.body_ptr)
    free_ffi_buffer(post_stored.content_type_ptr)

    @test post_out_body == UInt8[0x10, 0x20, 0xFF]
    @test post_out_content_type == "application/octet-stream"
    @test Fomalhaut._parse_headers(String(post_headers))["x-test"] == "1"

    options_method = UInt8['O', 'P', 'T', 'I', 'O', 'N', 'S']
    options_path = Vector{UInt8}(codeunits("/infer"))
    options_query = UInt8[]
    options_headers = Vector{UInt8}(codeunits("origin: http://localhost:5173\r\n"))
    options_body = UInt8[]
    options_response = Ref(Fomalhaut.FFIHttpResponse(Ptr{UInt8}(C_NULL), 0, Ptr{UInt8}(C_NULL), 0, 0))

    options_status = Fomalhaut._http_request_trampoline(
        C_NULL,
        pointer(options_method),
        Csize_t(length(options_method)),
        pointer(options_path),
        Csize_t(length(options_path)),
        pointer(options_query),
        Csize_t(length(options_query)),
        pointer(options_headers),
        Csize_t(length(options_headers)),
        pointer(options_body),
        Csize_t(length(options_body)),
        Base.unsafe_convert(Ptr{Fomalhaut.FFIHttpResponse}, options_response),
    )

    @test options_status == 0
    options_stored = options_response[]
    options_out_content_type = String(copy(unsafe_wrap(Vector{UInt8}, options_stored.content_type_ptr, Int(options_stored.content_type_len))))
    free_ffi_buffer(options_stored.content_type_ptr)

    @test options_stored.status_code == 204
    @test options_stored.body_len == 0
    @test options_out_content_type == "text/plain"

    Fomalhaut._active_app[] = nothing
end
