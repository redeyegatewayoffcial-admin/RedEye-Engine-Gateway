import React, { createContext, useContext, useState, useCallback } from 'react';

interface IncidentContextType {
  isIncidentActive: boolean;
  toggleIncident: () => void;
}

const IncidentContext = createContext<IncidentContextType | undefined>(undefined);

export function IncidentProvider({ children }: { children: React.ReactNode }) {
  const [isIncidentActive, setIsIncidentActive] = useState(false);

  const toggleIncident = useCallback(() => {
    setIsIncidentActive((prev) => !prev);
  }, []);

  return (
    <IncidentContext.Provider value={{ isIncidentActive, toggleIncident }}>
      {children}
    </IncidentContext.Provider>
  );
}

export function useIncident() {
  const context = useContext(IncidentContext);
  if (context === undefined) {
    throw new Error('useIncident must be used within an IncidentProvider');
  }
  return context;
}
