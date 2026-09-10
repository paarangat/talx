interface StatCardProps {
  label: string;
  value: string;
  unit?: string;
}

/**
 * One reading on the stat rail beneath the headline figure.
 *
 * These used to be six identical boxes in a 3×2 grid, which gave the day's
 * word count exactly the same weight as the streak counter and left the page
 * with no focal point at all.
 */
export const StatCard = ({ label, value, unit }: StatCardProps) => {
  return (
    <div className="stat">
      <span className="stat__value">
        {value}
        {unit && <span className="stat__unit">{unit}</span>}
      </span>
      <span className="stat__label">{label}</span>
    </div>
  );
};
