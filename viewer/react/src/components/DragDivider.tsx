import React from "react";

interface DragDividerProps {
  heightPercentage: number;
  setHeightPercentage: (heightPercentage: number) => void;
}

const DragDivider: React.FC<DragDividerProps> = ({
  heightPercentage,
  setHeightPercentage,
}) => {
  const [isDragging, setIsDragging] = React.useState(false);

  const handleMouseDown = () => {
    setIsDragging(true);
  };

  React.useEffect(() => {
    const onMouseMove = (moveEvent: MouseEvent) => {
      if (isDragging) {
        const newHeightPercentage =
          (moveEvent.clientY / window.innerHeight) * 100;
        setHeightPercentage(newHeightPercentage);
      }
    };

    const onMouseUp = () => {
      setIsDragging(false);
    };

    if (isDragging) {
      window.addEventListener("mousemove", onMouseMove);
      window.addEventListener("mouseup", onMouseUp);
    }

    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };
  }, [isDragging, setHeightPercentage]);

  return (
    <>
      {isDragging && (
        <div
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            width: "100vw",
            height: "100vh",
            cursor: "ns-resize",
            zIndex: 9999,
          }}
        />
      )}
      <div
        style={{
          height: "4px",
          width: "100vw",
          backgroundColor: "lightgray",
          cursor: "ns-resize",
          top: `${heightPercentage}%`,
        }}
        onMouseDown={handleMouseDown}
      />
    </>
  );
};

export default DragDivider;
