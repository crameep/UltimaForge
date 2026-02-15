import "./Sidebar.css";

interface SidebarLink {
  label: string;
  href?: string;
  onClick?: () => void;
  icon?: string;
}

interface SidebarProps {
  /** Server/product name to display */
  serverName?: string;
  /** Logo URL (optional) */
  logoUrl?: string;
  /** Navigation links */
  links?: SidebarLink[];
}

/**
 * Sidebar component for navigation and branding.
 * Displays the server logo, name, and navigation links.
 */
export function Sidebar({
  serverName = "UltimaForge",
  logoUrl,
  links = [],
}: SidebarProps) {
  // Default links if none provided
  const defaultLinks: SidebarLink[] = [
    { label: "Home", icon: "🏠" },
    { label: "Settings", icon: "⚙️" },
    { label: "Help", icon: "❓" },
  ];

  const navLinks = links.length > 0 ? links : defaultLinks;

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <div className="sidebar-logo">
          {logoUrl ? (
            <img src={logoUrl} alt={`${serverName} logo`} />
          ) : (
            <div className="sidebar-logo-placeholder">
              <span>⚔️</span>
            </div>
          )}
        </div>
        <h1 className="sidebar-title">{serverName}</h1>
        <p className="sidebar-subtitle">Self-Hosted UO Launcher</p>
      </div>

      <nav className="sidebar-nav">
        {navLinks.map((link, index) => (
          <a
            key={index}
            href={link.href || "#"}
            className="sidebar-link"
            onClick={(e) => {
              if (link.onClick) {
                e.preventDefault();
                link.onClick();
              }
            }}
          >
            {link.icon && <span className="sidebar-link-icon">{link.icon}</span>}
            <span>{link.label}</span>
          </a>
        ))}
      </nav>

      <div className="sidebar-footer">
        <a
          href="https://github.com/your-repo/ultimaforge"
          target="_blank"
          rel="noopener noreferrer"
          className="sidebar-footer-link"
        >
          Powered by UltimaForge
        </a>
      </div>
    </aside>
  );
}
