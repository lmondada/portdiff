import "./button.css";

type CommitButtonProps = {
    onClick: () => void;
};

function CommitButton(props: CommitButtonProps) {
    return (
        <button className="panel-button mx-2" {...props}>
            Commit selection
        </button>
    );
}

export default CommitButton;
