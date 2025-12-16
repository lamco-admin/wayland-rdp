//! EGFX Surface Management
//!
//! Manages RDP graphics surfaces for EGFX channel. Each surface represents
//! a rendering target that can be mapped to a monitor output.

use std::collections::HashMap;
use tracing::debug;

/// Information about an EGFX surface
#[derive(Debug, Clone)]
pub struct EgfxSurface {
    /// Surface ID (assigned by server)
    pub id: u16,
    /// Surface width in pixels
    pub width: u16,
    /// Surface height in pixels
    pub height: u16,
    /// Whether this surface is mapped to an output
    pub is_mapped: bool,
    /// Output coordinates if mapped (x, y)
    pub output_origin: Option<(u32, u32)>,
}

impl EgfxSurface {
    /// Create a new surface
    pub fn new(id: u16, width: u16, height: u16) -> Self {
        Self {
            id,
            width,
            height,
            is_mapped: false,
            output_origin: None,
        }
    }

    /// Map this surface to an output at the given origin
    pub fn map_to_output(&mut self, x: u32, y: u32) {
        self.is_mapped = true;
        self.output_origin = Some((x, y));
    }

    /// Unmap this surface from output
    pub fn unmap(&mut self) {
        self.is_mapped = false;
        self.output_origin = None;
    }
}

/// Manages multiple EGFX surfaces
///
/// In a single-monitor setup, there's typically one primary surface.
/// Multi-monitor setups may have multiple surfaces, one per monitor.
#[derive(Debug)]
pub struct SurfaceManager {
    /// All surfaces by ID
    surfaces: HashMap<u16, EgfxSurface>,
    /// Next surface ID to assign
    next_id: u16,
    /// Primary surface ID (if set)
    primary_id: Option<u16>,
}

impl SurfaceManager {
    /// Create a new surface manager
    pub fn new() -> Self {
        Self {
            surfaces: HashMap::new(),
            next_id: 1,
            primary_id: None,
        }
    }

    /// Create a new surface and return its ID
    ///
    /// The first surface created becomes the primary surface.
    pub fn create_surface(&mut self, width: u16, height: u16) -> u16 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let surface = EgfxSurface::new(id, width, height);
        debug!("Created EGFX surface {}: {}x{}", id, width, height);

        self.surfaces.insert(id, surface);

        // First surface becomes primary
        if self.primary_id.is_none() {
            self.primary_id = Some(id);
        }

        id
    }

    /// Delete a surface by ID
    pub fn delete_surface(&mut self, id: u16) -> Option<EgfxSurface> {
        let surface = self.surfaces.remove(&id);
        if self.primary_id == Some(id) {
            // Pick new primary if available
            self.primary_id = self.surfaces.keys().next().copied();
        }
        surface
    }

    /// Get a surface by ID
    pub fn get_surface(&self, id: u16) -> Option<&EgfxSurface> {
        self.surfaces.get(&id)
    }

    /// Get a mutable surface by ID
    pub fn get_surface_mut(&mut self, id: u16) -> Option<&mut EgfxSurface> {
        self.surfaces.get_mut(&id)
    }

    /// Get the primary surface
    pub fn primary_surface(&self) -> Option<&EgfxSurface> {
        self.primary_id.and_then(|id| self.surfaces.get(&id))
    }

    /// Get the primary surface mutably
    pub fn primary_surface_mut(&mut self) -> Option<&mut EgfxSurface> {
        self.primary_id.and_then(|id| self.surfaces.get_mut(&id))
    }

    /// Set the primary surface
    pub fn set_primary(&mut self, id: u16) {
        if self.surfaces.contains_key(&id) {
            self.primary_id = Some(id);
        }
    }

    /// Get all surfaces
    pub fn surfaces(&self) -> impl Iterator<Item = &EgfxSurface> {
        self.surfaces.values()
    }

    /// Get number of surfaces
    pub fn len(&self) -> usize {
        self.surfaces.len()
    }

    /// Check if there are no surfaces
    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
    }

    /// Clear all surfaces
    pub fn clear(&mut self) {
        self.surfaces.clear();
        self.primary_id = None;
        debug!("Cleared all EGFX surfaces");
    }

    /// Resize the primary surface
    ///
    /// Note: This only updates our tracking. The actual resize requires
    /// deleting and recreating the surface on the client side.
    pub fn resize_primary(&mut self, width: u16, height: u16) -> Option<u16> {
        if let Some(surface) = self.primary_surface_mut() {
            surface.width = width;
            surface.height = height;
            debug!("Resized primary surface to {}x{}", width, height);
            Some(surface.id)
        } else {
            None
        }
    }
}

impl Default for SurfaceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_creation() {
        let mut manager = SurfaceManager::new();

        let id1 = manager.create_surface(1920, 1080);
        assert_eq!(id1, 1);
        assert_eq!(manager.len(), 1);

        let surface = manager.get_surface(id1).unwrap();
        assert_eq!(surface.width, 1920);
        assert_eq!(surface.height, 1080);
        assert!(!surface.is_mapped);
    }

    #[test]
    fn test_primary_surface() {
        let mut manager = SurfaceManager::new();

        let id1 = manager.create_surface(1920, 1080);
        let id2 = manager.create_surface(1280, 720);

        // First surface is primary
        assert_eq!(manager.primary_surface().unwrap().id, id1);

        // Can change primary
        manager.set_primary(id2);
        assert_eq!(manager.primary_surface().unwrap().id, id2);
    }

    #[test]
    fn test_surface_mapping() {
        let mut manager = SurfaceManager::new();
        let id = manager.create_surface(1920, 1080);

        let surface = manager.get_surface_mut(id).unwrap();
        assert!(!surface.is_mapped);

        surface.map_to_output(0, 0);
        assert!(surface.is_mapped);
        assert_eq!(surface.output_origin, Some((0, 0)));

        surface.unmap();
        assert!(!surface.is_mapped);
        assert_eq!(surface.output_origin, None);
    }

    #[test]
    fn test_surface_deletion() {
        let mut manager = SurfaceManager::new();

        let id1 = manager.create_surface(1920, 1080);
        let id2 = manager.create_surface(1280, 720);

        assert_eq!(manager.len(), 2);

        // Delete primary
        manager.delete_surface(id1);
        assert_eq!(manager.len(), 1);

        // New primary should be id2
        assert_eq!(manager.primary_surface().unwrap().id, id2);
    }
}
