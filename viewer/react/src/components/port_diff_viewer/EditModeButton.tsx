import React from "react";

interface EditModeButtonProps {
    isEditMode: boolean;
    toggleEditMode: () => void;
}

const EditModeButton: React.FC<EditModeButtonProps> = ({
    isEditMode,
    toggleEditMode,
}) => {
    return (
        <button
            onClick={toggleEditMode}
            style={{
                backgroundColor: "#fafafa",
                border: "1px solid #ccc",
                borderRadius: "5px",
                padding: "10px 20px",
                cursor: "pointer",
            }}
        >
            {isEditMode ? "Commit & Leave Edit Mode" : "Enter Edit Mode"}
        </button>
    );
};

export default EditModeButton;
