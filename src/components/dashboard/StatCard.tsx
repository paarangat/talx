interface StatCardProps {
  label: string;
  value: string;
  unit?: string;
}

export const StatCard = ({ label, value, unit }: StatCardProps) => {
  return (
    <div className="stat-card">
      <span className="stat-card__value">
        {value}
        {unit && <span className="stat-card__unit">{unit}</span>}
      </span>
      <span className="stat-card__label">{label}</span>
    </div>
  );
};
