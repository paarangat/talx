import { useState, useCallback } from "react";

export const useFirstUse = (key: string): [boolean, () => void] => {
  const storageKey = `talx_first_use_${key}`;
  const [isFirst, setIsFirst] = useState(() => {
    try {
      return localStorage.getItem(storageKey) === null;
    } catch {
      return false;
    }
  });

  const markUsed = useCallback(() => {
    try {
      localStorage.setItem(storageKey, "1");
    } catch (err: unknown) {
      console.error("Failed to write to localStorage:", err);
    }
    setIsFirst(false);
  }, [storageKey]);

  return [isFirst, markUsed];
};
