import React from "react";

import PortDiffViewer from "./components/PortDiffViewer";

import "./App.css";
import "tailwindcss/tailwind.css";
import "reactflow/dist/style.css";
import DragDivider from "./components/DragDivider";

const App = () => {
    const [widthPercentage, setWidthPercentage] = React.useState(50);

    const handleSliderChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        setWidthPercentage(Number(event.target.value));
    };

    return (
        <div style={{ display: "flex", width: "100vw", height: "100vh" }}>
            <div style={{ width: `${widthPercentage - 2}%`, height: "100%" }}>
                <PortDiffViewer />
            </div>
            <DragDivider
                widthPercentage={widthPercentage}
                setWidthPercentage={setWidthPercentage}
            />
            <div
                style={{
                    width: `${100 - widthPercentage - 2}%`,
                    height: "100%",
                }}
            >
                {/* Another component can be placed here */}
            </div>
        </div>
    );
    // return (
    //     <div style={{ width: "50vw", height: "100vh" }}>
    //         <PortDiff />
    //     </div>
    // );
};

export default App;
