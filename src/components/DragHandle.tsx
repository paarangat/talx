export const DragHandle = () => {
  return (
    <div className="drag-handle">
      <div className="drag-handle__dots">
        {Array.from({ length: 6 }, (_, i) => (
          <div key={i} className="drag-handle__dot" />
        ))}
      </div>
    </div>
  );
};
