import { useCallback, useState } from "react";

import PortDiffViewer from "./components/PortDiffViewer";

import "./App.css";
import "tailwindcss/tailwind.css";
import "reactflow/dist/style.css";
import DragDivider from "./components/DragDivider";
import HierarchyViewer from "./components/HierarchyViewer";

function useCommunicationChannel(): [boolean, () => void] {
    const [flag, setFlag] = useState(false);
    const sendFlag = useCallback(() => setFlag((f) => !f), [setFlag]);
    return [flag, sendFlag];
}

const App = () => {
    const [widthPercentage, setWidthPercentage] = useState(70);
    // Communicate state update between PortDiffViewer and HierarchyViewer
    const [updatePortDiff, sendUpdatePortDiff] = useCommunicationChannel();
    const [updateHierarchy, sendUpdateHierarchy] = useCommunicationChannel();

    return (
        <div style={{ display: "flex", width: "100vw", height: "100vh" }}>
            <div style={{ width: `${widthPercentage - 2}%`, height: "100%" }}>
                <PortDiffViewer
                    updatePortDiff={updatePortDiff}
                    sendUpdateHierarchy={sendUpdateHierarchy}
                />
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
                <HierarchyViewer
                    updateHierarchy={updateHierarchy}
                    sendUpdatePortDiff={sendUpdatePortDiff}
                />
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
