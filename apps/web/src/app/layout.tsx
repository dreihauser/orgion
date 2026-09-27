import type { Metadata } from "next";
import "./globals.css";
import { AuthProvider } from "@/lib/auth";
import { ThemeProvider, themeInitScript } from "@/lib/theme";

export const metadata: Metadata = {
  title: "Orgion",
  description: "Org-mode based local-first workspace",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    // suppressHydrationWarning here guards against browser extensions
    // (translators, Grammarly, dark-mode injectors, etc.) that mutate
    // <html>/<body> attributes before React hydrates — not an app bug.
    <html lang="en" suppressHydrationWarning>
      <head>
        {/* Applies the saved theme before first paint, so there's no
            flash of the wrong theme while React hydrates. */}
        <script dangerouslySetInnerHTML={{ __html: themeInitScript }} />
      </head>
      <body className="min-h-screen" suppressHydrationWarning>
        <ThemeProvider>
          <AuthProvider>{children}</AuthProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
