import React from "react";

interface CircuitViewerProps {
  circuitJson: string;
}

export default function CircuitViewer({ circuitJson }: CircuitViewerProps) {
  const iframeRef = React.useRef<HTMLIFrameElement>(null);

  React.useEffect(() => {
    const iframe = iframeRef.current;
    if (iframe && iframe.contentWindow) {
      console.log("sending message");
      iframe.contentWindow.postMessage(
        {
          type: "updateCircuit",
          circuitJson,
        },
        "*"
      );
    }
  }, [circuitJson, iframeRef]);

  return (
    <iframe
      ref={iframeRef}
      src="/circuit_viewer.html"
      style={{ width: "100%", height: "100%", border: "none" }}
      title="Circuit Viewer"
    />
  );
}
