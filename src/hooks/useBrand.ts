/**
 * Hook for loading and accessing brand configuration.
 */

import { useState, useEffect } from "react";
import { getBrandConfig, type BrandInfo } from "../lib/api";

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
        setBrandInfo(brand);
        setError(null);
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
          show_patch_notes: true,
          window_title: "UltimaForge Launcher",
        });
      } finally {
        setLoading(false);
      }
    };

    loadBrand();
  }, []);

  return { brandInfo, loading, error };
}
