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
  /** Installed game client version */
  clientVersion?: string | null;
  /** Number of running clients */
  runningClients?: number;
  /** Custom sidebar navigation links */
  sidebarLinks?: SidebarLink[];
  /** Callback when settings is clicked */
  onSettingsClick?: () => void;
  /** Callback when home is clicked */
  onHomeClick?: () => void;
  /** Callback when launch options is clicked */
  onLaunchOptionsClick?: () => void;
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
  clientVersion,
  sidebarLinks,
  onSettingsClick,
  onHomeClick,
  onLaunchOptionsClick,
  runningClients,
}: LayoutProps) {
  const { brandInfo } = useBrand();

  // Build default links with navigation callbacks
  const defaultLinks: SidebarLink[] = [
    { label: "Home", icon: "🏠", onClick: onHomeClick },
    { label: "Launch Options", icon: "🎮", onClick: onLaunchOptionsClick },
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
        subtitle={brandInfo?.sidebar_subtitle || undefined}
        backgroundUrl={brandInfo?.sidebar_background || undefined}
        links={brandInfo?.sidebar_links?.length ? (() => {
          const mapped = brandInfo.sidebar_links!.map(link => ({
            label: link.label,
            icon: link.icon,
            href: link.url,
            url: link.url,
            onClick: link.label === "Home" ? onHomeClick : link.label === "Settings" ? onSettingsClick : link.label === "Launch Options" ? onLaunchOptionsClick : undefined,
          }));
          // Inject Launch Options before Settings if not already present
          if (!mapped.some(l => l.label === "Launch Options")) {
            const settingsIdx = mapped.findIndex(l => l.label === "Settings");
            const entry = { label: "Launch Options", icon: "🎮", onClick: onLaunchOptionsClick };
            if (settingsIdx >= 0) {
              mapped.splice(settingsIdx, 0, entry);
            } else {
              mapped.push(entry);
            }
          }
          return mapped;
        })() : links}
      />
      <main className={`layout-main${brandInfo?.background_image ? " has-background" : ""}`} style={mainStyle}>
        <div className="layout-content">{children}</div>
      </main>
      <StatusBar
        phase={phase}
        message={statusMessage}
        version={version}
        clientVersion={clientVersion}
        runningClients={runningClients}
      />
    </div>
  );
}
