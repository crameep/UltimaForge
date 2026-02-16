import { ReactNode } from "react";
import { Sidebar } from "./Sidebar";
import { StatusBar } from "./StatusBar";
import { useBrand } from "../hooks/useBrand";
import "./Layout.css";

/** Link configuration for sidebar navigation. */
interface SidebarLink {
  label: string;
  href?: string;
  onClick?: () => void;
  icon?: string;
}

interface LayoutProps {
  children: ReactNode;
  /** Current application phase for status bar */
  phase?: string;
  /** Status message to display */
  statusMessage?: string;
  /** Current version string */
  version?: string;
  /** Custom sidebar navigation links */
  sidebarLinks?: SidebarLink[];
  /** Callback when settings is clicked */
  onSettingsClick?: () => void;
  /** Callback when home is clicked */
  onHomeClick?: () => void;
}

/**
 * Main application layout component.
 * Provides the overall structure: sidebar, main content area, and status bar.
 */
export function Layout({
  children,
  phase = "Ready",
  statusMessage,
  version,
  sidebarLinks,
  onSettingsClick,
  onHomeClick,
}: LayoutProps) {
  const { brandInfo } = useBrand();

  // Build default links with navigation callbacks
  const defaultLinks: SidebarLink[] = [
    { label: "Home", icon: "🏠", onClick: onHomeClick },
    { label: "Settings", icon: "⚙️", onClick: onSettingsClick },
    { label: "Help", icon: "❓" },
  ];

  const links = sidebarLinks || defaultLinks;

  // Apply background image from brand config
  const mainStyle: React.CSSProperties = brandInfo?.background_image
    ? {
        backgroundImage: `url(${brandInfo.background_image})`,
        backgroundSize: "cover",
        backgroundPosition: "center",
        backgroundRepeat: "no-repeat",
      }
    : {};

  return (
    <div className="layout">
      <Sidebar
        serverName={brandInfo?.display_name}
        logoUrl={brandInfo?.logo_url || undefined}
        links={links}
      />
      <main className="layout-main" style={mainStyle}>
        <div className="layout-content">{children}</div>
      </main>
      <StatusBar phase={phase} message={statusMessage} version={version} />
    </div>
  );
}
