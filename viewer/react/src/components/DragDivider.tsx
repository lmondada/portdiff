import React from "react";

interface DragDividerProps {
    widthPercentage: number;
    setWidthPercentage: (widthPercentage: number) => void;
}

const DragDivider: React.FC<DragDividerProps> = ({
    widthPercentage,
    setWidthPercentage,
}) => {
    const [isDragging, setIsDragging] = React.useState(false);

    const handleMouseDown = () => {
        setIsDragging(true);
    };

    React.useEffect(() => {
        const onMouseMove = (moveEvent: MouseEvent) => {
            if (isDragging) {
                const newWidthPercentage =
                    (moveEvent.clientX / window.innerWidth) * 100;
                setWidthPercentage(newWidthPercentage);
            }
        };

        const onMouseUp = () => {
            setIsDragging(false);
        };

        if (isDragging) {
            window.addEventListener("mousemove", onMouseMove);
            window.addEventListener("mouseup", onMouseUp);
        } else {
            window.removeEventListener("mousemove", onMouseMove);
            window.removeEventListener("mouseup", onMouseUp);
        }

        return () => {
            window.removeEventListener("mousemove", onMouseMove);
            window.removeEventListener("mouseup", onMouseUp);
        };
    }, [isDragging, setWidthPercentage]);

    return (
        <div
            style={{
                width: "4px",
                height: "100vh",
                backgroundColor: "lightgray",
                cursor: "ew-resize",
                position: "absolute",
                left: `${widthPercentage}%`,
            }}
            onMouseDown={handleMouseDown}
        />
    );
};

export default DragDivider;
