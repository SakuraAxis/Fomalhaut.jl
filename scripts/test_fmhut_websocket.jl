import Fomalhaut as FMHUT

const RES = 96
const BUFFER = zeros(Float32, RES, RES)
const R = range(-3f0, 3f0, length=RES)

function wave_stream(ctx)
    t = Float32(ctx.time * 2.0)
    BUFFER .= sin.(R .+ t) .+ cos.(R' .+ t)

    return vec(BUFFER)
end

app = FMHUT.App()

@FMHUT.websocket app "/live-wave" wave_stream

FMHUT.serve(app; port=8080, fps=60)

#=
Frontend Usage Example :

const ws = new WebSocket("ws://127.0.0.1:8080/live-wave");
ws.binaryType = "arraybuffer";

ws.onopen = () => {
  console.log("WebSocket connected");
};

ws.onmessage = (event) => {
  const frame = new Uint8Array(event.data);

  console.log("Frame Bytes :", frame.byteLength);

  const version = frame[0];
  const contentType = new DataView(frame.buffer).getUint16(1, true);
  const payloadLength = new DataView(frame.buffer).getUint32(13, true);
  const payload = frame.slice(17, 17 + payloadLength);
  const tensor = new Float32Array(
    payload.buffer,
    payload.byteOffset,
    payload.byteLength / 4
  );

  console.log("Envelope Version :", version);
  console.log("Content Type ( Expected 1 ) :", contentType);
  console.log("Float32 Count ( Expected 9216 ) :", tensor.length);
  console.log("First 8 Values :", Array.from(tensor.slice(0, 8)));
};

ws.onerror = (err) => {
  console.error("WebSocket error :", err);
};

ws.onclose = () => {
  console.log("WebSocket closed");
};
=#
