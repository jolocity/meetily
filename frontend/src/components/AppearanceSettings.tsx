"use client"

import { Monitor, Moon, Sun, LucideIcon } from "lucide-react"
import { useTheme } from "@/contexts/ThemeContext"
import { ThemePreference } from "@/lib/theme"
import { cn } from "@/lib/utils"

const THEME_OPTIONS: { value: ThemePreference; label: string; icon: LucideIcon }[] = [
  { value: 'light', label: 'Light', icon: Sun },
  { value: 'dark', label: 'Dark', icon: Moon },
  { value: 'system', label: 'System', icon: Monitor },
]

export function AppearanceSettings() {
  const { preference, setPreference } = useTheme();

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
      <h3 className="text-lg font-semibold text-gray-900 mb-2">Appearance</h3>
      <p className="text-sm text-gray-600 mb-4">Choose a light or dark theme, or follow your system setting</p>

      <div role="radiogroup" aria-label="Theme" className="grid grid-cols-3 gap-3 max-w-md">
        {THEME_OPTIONS.map(({ value, label, icon: Icon }) => {
          const isSelected = preference === value;
          return (
            <button
              key={value}
              type="button"
              role="radio"
              aria-checked={isSelected}
              onClick={() => setPreference(value)}
              className={cn(
                "flex flex-col items-center gap-2 rounded-lg border p-3 text-sm font-medium transition-colors",
                isSelected
                  ? "border-blue-500 bg-blue-50 text-blue-700"
                  : "border-gray-200 text-gray-700 hover:bg-gray-50"
              )}
            >
              <Icon className="h-5 w-5" />
              {label}
            </button>
          );
        })}
      </div>
    </div>
  )
}
