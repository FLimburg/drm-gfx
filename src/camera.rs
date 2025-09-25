use std::f32::consts;

use nalgebra::{Isometry3, Perspective3, Point3, Vector3};

pub struct Camera {
    pub position: Point3<f32>,
    fov: f32,
    pub near: f32,
    pub far: f32,
    view_matrix: nalgebra::Matrix4<f32>,
    projection_matrix: nalgebra::Matrix4<f32>,
    pub vp_matrix: nalgebra::Matrix4<f32>,
    target: Point3<f32>,
    aspect_ratio: f32,
}

impl Camera {
    pub fn new(aspect_ratio: f32) -> Camera {
        let mut ret = Camera {
            position: Point3::new(0.0, 0.0, 0.0),
            fov: consts::PI / 2.0,
            view_matrix: nalgebra::Matrix4::identity(),
            projection_matrix: nalgebra::Matrix4::identity(),
            vp_matrix: nalgebra::Matrix4::identity(),
            target: Point3::new(0.0, 0.0, 0.0),
            aspect_ratio,
            near: 0.4,
            far: 20.0,
        };

        ret.update_projection();

        ret
    }

    pub fn set_position(&mut self, pos: Point3<f32>) -> &Self {
        self.position = pos;

        self.update_view();
        self
    }

    pub fn set_near_far(&mut self, near: f32, far: f32) -> &Self {
        self.near = near;
        self.far = far;

        self.update_projection()
    }

    pub fn set_fovy(&mut self, fovy: f32) -> &Self {
        self.fov = fovy;

        self.update_projection()
    }

    pub fn set_target(&mut self, target: Point3<f32>) -> &Self {
        self.target = target;
        self.update_view()
    }

    pub fn get_direction(&self) -> Vector3<f32> {
        // Get direction from position to target and normalize it
        let dir = self.target - self.position;
        dir.normalize()
    }

    fn update_view(&mut self) -> &Self {
        let view = Isometry3::look_at_rh(&self.position, &self.target, &Vector3::y());

        self.view_matrix = view.to_homogeneous();
        self.vp_matrix = self.projection_matrix * self.view_matrix;
        self
    }

    fn update_projection(&mut self) -> &Self {
        let projection = Perspective3::new(self.aspect_ratio, self.fov, self.near, self.far);
        self.projection_matrix = projection.to_homogeneous();
        self.vp_matrix = self.projection_matrix * self.view_matrix;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{Point3, Vector3};
    use std::f32::consts::PI;

    /// Helper function to compare floating point values with a tolerance
    fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
        (a - b).abs() < epsilon
    }

    /// Helper function to compare points with a tolerance
    fn points_approx_eq(a: Point3<f32>, b: Point3<f32>, epsilon: f32) -> bool {
        approx_eq(a.x, b.x, epsilon) && 
        approx_eq(a.y, b.y, epsilon) && 
        approx_eq(a.z, b.z, epsilon)
    }

    /// Helper function to compare matrices with a tolerance
    fn matrices_approx_eq(a: &nalgebra::Matrix4<f32>, b: &nalgebra::Matrix4<f32>, epsilon: f32) -> bool {
        for i in 0..4 {
            for j in 0..4 {
                if !approx_eq(a[(i, j)], b[(i, j)], epsilon) {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn test_camera_creation() {
        let aspect_ratio = 16.0 / 9.0;
        let camera = Camera::new(aspect_ratio);

        // Check default values
        assert!(points_approx_eq(camera.position, Point3::new(0.0, 0.0, 0.0), 0.001));
        assert!(points_approx_eq(camera.target, Point3::new(0.0, 0.0, 0.0), 0.001));
        assert!(approx_eq(camera.fov, PI / 2.0, 0.001));
        assert!(approx_eq(camera.near, 0.4, 0.001));
        assert!(approx_eq(camera.far, 20.0, 0.001));
        assert!(approx_eq(camera.aspect_ratio, aspect_ratio, 0.001));
        
        // Identity matrices as default
        let identity = nalgebra::Matrix4::identity();
        assert!(matrices_approx_eq(&camera.view_matrix, &identity, 0.001));
    }

    #[test]
    fn test_set_position() {
        let mut camera = Camera::new(1.0);
        let new_position = Point3::new(1.0, 2.0, 3.0);
        
        camera.set_position(new_position);
        
        // Check that position was updated
        assert!(points_approx_eq(camera.position, new_position, 0.001));
        
        // View matrix should no longer be identity
        let identity = nalgebra::Matrix4::identity();
        assert!(!matrices_approx_eq(&camera.view_matrix, &identity, 0.001));
        
        // VP matrix should also be updated
        assert!(!matrices_approx_eq(&camera.vp_matrix, &identity, 0.001));
    }

    #[test]
    fn test_set_target() {
        let mut camera = Camera::new(1.0);
        camera.set_position(Point3::new(0.0, 0.0, 10.0));
        
        let new_target = Point3::new(0.0, 0.0, 0.0);
        camera.set_target(new_target);
        
        // Check that target was updated
        assert!(points_approx_eq(camera.target, new_target, 0.001));
        
        // Direction should be pointing toward -Z (from position at (0,0,10) to target at origin)
        let direction = camera.get_direction();
        let expected_direction = Vector3::new(0.0, 0.0, -1.0);
        
        assert!(approx_eq(direction.x, expected_direction.x, 0.001));
        assert!(approx_eq(direction.y, expected_direction.y, 0.001));
        assert!(approx_eq(direction.z, expected_direction.z, 0.001));
    }
    
    #[test]
    fn test_set_near_far() {
        let mut camera = Camera::new(1.0);
        let new_near = 1.0;
        let new_far = 100.0;
        
        camera.set_near_far(new_near, new_far);
        
        // Check that near and far were updated
        assert!(approx_eq(camera.near, new_near, 0.001));
        assert!(approx_eq(camera.far, new_far, 0.001));
        
        // Projection matrix should be updated
        let before_projection = camera.projection_matrix.clone();
        camera.set_near_far(2.0, 200.0);
        assert!(!matrices_approx_eq(&camera.projection_matrix, &before_projection, 0.001));
    }
    
    #[test]
    fn test_set_fovy() {
        let mut camera = Camera::new(1.0);
        let new_fov = PI / 4.0;
        
        camera.set_fovy(new_fov);
        
        // Check that FOV was updated
        assert!(approx_eq(camera.fov, new_fov, 0.001));
        
        // Projection matrix should be updated
        let before_projection = camera.projection_matrix.clone();
        camera.set_fovy(PI / 3.0);
        assert!(!matrices_approx_eq(&camera.projection_matrix, &before_projection, 0.001));
    }
    
    #[test]
    fn test_method_chaining() {
        let mut camera = Camera::new(1.0);
        
        // Since methods return &self, we need to call them separately
        camera.set_position(Point3::new(1.0, 2.0, 3.0));
        camera.set_target(Point3::new(0.0, 0.0, 0.0));
        camera.set_near_far(0.1, 100.0);
        camera.set_fovy(PI / 3.0);
            
        // Verify all settings were applied
        assert!(points_approx_eq(camera.position, Point3::new(1.0, 2.0, 3.0), 0.001));
        assert!(points_approx_eq(camera.target, Point3::new(0.0, 0.0, 0.0), 0.001));
        assert!(approx_eq(camera.near, 0.1, 0.001));
        assert!(approx_eq(camera.far, 100.0, 0.001));
        assert!(approx_eq(camera.fov, PI / 3.0, 0.001));
    }
    
    #[test]
    fn test_get_direction() {
        let mut camera = Camera::new(1.0);
        
        // Test with camera at origin looking along -Z axis
        camera.set_position(Point3::new(0.0, 0.0, 0.0));
        camera.set_target(Point3::new(0.0, 0.0, -1.0));
        
        let direction = camera.get_direction();
        assert!(approx_eq(direction.x, 0.0, 0.001));
        assert!(approx_eq(direction.y, 0.0, 0.001));
        assert!(approx_eq(direction.z, -1.0, 0.001));
        
        // Test with camera at (10,0,0) looking at origin
        camera.set_position(Point3::new(10.0, 0.0, 0.0));
        camera.set_target(Point3::new(0.0, 0.0, 0.0));
        
        let direction = camera.get_direction();
        // Direction should be normalized, pointing from (10,0,0) to (0,0,0)
        // So it should be approximately (-1, 0, 0)
        assert!(approx_eq(direction.x, -1.0, 0.001));
        assert!(approx_eq(direction.y, 0.0, 0.001));
        assert!(approx_eq(direction.z, 0.0, 0.001));
    }
    
    #[test]
    fn test_look_at_matrix() {
        let mut camera = Camera::new(1.0);
        
        // Position camera at origin
        camera.set_position(Point3::new(0.0, 0.0, 0.0));
        camera.set_target(Point3::new(0.0, 0.0, -1.0));
        
        // Create an object position
        let object_position = Point3::new(0.0, 0.0, -5.0);
        
        // Transform object position by view matrix
        let transformed = camera.view_matrix.transform_point(&object_position);
        
        // Object should be in front of camera on z-axis
        assert!(approx_eq(transformed.x, 0.0, 0.001));
        assert!(approx_eq(transformed.y, 0.0, 0.001));
        assert!(transformed.z < 0.0);
        
        // Now move camera and check if transformation still works
        camera.set_position(Point3::new(0.0, 0.0, 5.0));
        camera.set_target(Point3::new(0.0, 0.0, 0.0));
        
        // Transform object position by new view matrix
        let transformed = camera.view_matrix.transform_point(&object_position);
        
        // Object should still be in front of camera
        assert!(transformed.z < 0.0);
    }
}
