"""Regression checks for eye triangles and cheek paint hidden by an open eye."""
import unittest
import numpy as np
from face_regions import blush_layer, blush_profiles, eye_triangles


class FaceRegionsTests(unittest.TestCase):
    def test_eye_crossing_triangle_pins_all_vertices(self):
        face = dict(center_x=0., eye_x=.15, eye_y=1.4, eye_half_height=.08)
        # A large triangle crosses the eye even though its vertices are beyond
        # the landmark ellipse. The separate low cheek remains free to move.
        points = np.array([[-.28, -.2, 1.27], [-.02, -.2, 1.27], [-.15, -.2, 1.55],
                           [-.28, -.2, 1.14], [-.24, -.2, 1.14], [-.26, -.2, 1.18]])
        pinned = eye_triangles(points, np.array([[0, 1, 2], [3, 4, 5]]), face)
        np.testing.assert_array_equal(pinned, [True, True, True, False, False, False])

    def test_raised_eye_sheet_does_not_cut_the_blush_circle(self):
        pink, skin = [.99, .65, .72], [.99, .90, .88]
        profiles = {'L': dict(center=[-.2, 1.25], matrix=[[100., 0.], [0., 100.]], color=pink)}
        points = np.array([[-.2, -.15, 1.25], [-.2, -.31, 1.25], [.2, -.15, 1.25]])
        colors, coverage = blush_layer(points, profiles, skin)
        np.testing.assert_allclose(colors[:2], [pink, pink])
        np.testing.assert_allclose(colors[2], skin)
        np.testing.assert_array_equal(coverage, [1., 1., 0.])

    def test_blush_hidden_by_eye_recovers_the_lower_round_contour(self):
        xs, zs = np.meshgrid(np.linspace(-.4, .4, 256), np.linspace(1.05, 1.5, 256))
        pos = np.stack([xs, np.full_like(xs, -.2), zs], axis=-1)
        base = np.full((*xs.shape, 4), 1.)
        base[:, :, :3] = [.99, .95, .94]
        for x in [-.23, .23]:
            blush = ((xs-x)/.08)**2+((zs-1.25)/.09)**2 < 1
            base[blush, :3] = [.99, .70, .76]
            occluded = blush & (zs > 1.27) & (np.abs(xs-x) < .045)
            base[occluded, :3] = [.1, .1, .1]
        face = dict(center_x=0., eye_x=.14, eye_y=1.38, eye_half_height=.08, mouth_y=1.25)
        profiles = blush_profiles(pos, base, np.ones_like(xs, dtype=bool), face)
        self.assertEqual(set(profiles), {'L', 'R'})
        for side, x in [('L', -.23), ('R', .23)]:
            profile = profiles[side]
            np.testing.assert_allclose(profile['center'], [x, 1.25], atol=.006)
            np.testing.assert_allclose(1/np.sqrt(np.diag(profile['matrix'])), [.08, .09], atol=.006)


if __name__ == '__main__':
    unittest.main()
