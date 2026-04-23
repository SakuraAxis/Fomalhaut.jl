import Fomalhaut as FMHUT

app = FMHUT.App()

@FMHUT.post app "/echo" (req) -> begin
    return (UInt8[], "text/plain", 204)
end

FMHUT.serve(app; port=8080)

#=
Frontend Usage Example :

fetch("http://127.0.0.1:8080/echo", {
  method: "OPTIONS",
  headers: {
    "Origin": "http://localhost:5173",
    "Access-Control-Request-Method": "POST",
    "Access-Control-Request-Headers": "Content-Type, X-Custom-Header"
  }
})
.then(async res => {
  console.log("Status ( Expected 204 ) :", res.status);
  console.log("Allow-Origin :", res.headers.get("access-control-allow-origin"));
  console.log("Allow-Methods :", res.headers.get("access-control-allow-methods"));
  console.log("Allow-Headers :", res.headers.get("access-control-allow-headers"));
  console.log("Body Length ( Expected 0 ) :", (await res.text()).length);
});
=#
