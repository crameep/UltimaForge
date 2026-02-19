/**
 * Hook for loading and accessing brand configuration.
 */

import { useState, useEffect } from "react";
import { getBrandConfig, type BrandInfo } from "../lib/api";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * Hook to load and access brand configuration.
 *
 * @returns Brand information or null if not available.
 */
export function useBrand() {
  const [brandInfo, setBrandInfo] = useState<BrandInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadBrand = async () => {
      try {
        const brand = await getBrandConfig();

        // Images are served from embedded dist/branding/ folder
        // Paths like /branding/image.png work directly in both dev and production
        setBrandInfo(brand);
        setError(null);

        // Set window title from brand config
        try {
          const appWindow = getCurrentWindow();
          await appWindow.setTitle(brand.window_title);
        } catch (err) {
          console.warn("Failed to set window title:", err);
        }
      } catch (err) {
        console.warn("Failed to load brand configuration:", err);
        setError(err instanceof Error ? err.message : String(err));
        // Set default brand info for development
        setBrandInfo({
          display_name: "UltimaForge",
          server_name: "UltimaForge",
          description: null,
          support_email: null,
          website: null,
          discord: null,
          colors: {
            primary: "#1a1a2e",
            secondary: "#e94560",
            background: "#16213e",
            text: "#ffffff",
          },
          background_image: null,
          logo_url: null,
          sidebar_background: null,
          show_patch_notes: true,
          window_title: "UltimaForge Launcher",
          hero_title: null,
          hero_subtitle: null,
          sidebar_subtitle: null,
          sidebar_links: null,
        });
      } finally {
        setLoading(false);
      }
    };

    loadBrand();
  }, []);

  return { brandInfo, loading, error };
}
