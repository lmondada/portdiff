import React from "react";
import "./button.css";

interface EditModeButtonProps {
    isEditMode: boolean;
    toggleEditMode: () => void;
}

const EditModeButton: React.FC<EditModeButtonProps> = ({
    isEditMode,
    toggleEditMode,
}) => {
    return (
        <button onClick={toggleEditMode} className="panel-button">
            {isEditMode ? "Commit Rewrite" : "Enter Edit Mode"}
        </button>
    );
};

export default EditModeButton;
