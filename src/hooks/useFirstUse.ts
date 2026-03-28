import { useState, useCallback } from "react";

export const useFirstUse = (key: string): [boolean, () => void] => {
  const storageKey = `talx_first_use_${key}`;
  const [isFirst, setIsFirst] = useState(() => {
    return localStorage.getItem(storageKey) === null;
  });

  const markUsed = useCallback(() => {
    localStorage.setItem(storageKey, "1");
    setIsFirst(false);
  }, [storageKey]);

  return [isFirst, markUsed];
};
