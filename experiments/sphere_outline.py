import numpy as np
import matplotlib.pyplot as plt

# Should give half circle
pt1 = np.array([1.99999, 0])
pt2 = np.array([2, -1])

result = []

# Should be band size of half circle
distance = np.sum((pt2 - pt1)**2)**0.5

y_intersect = -pt1[0] * ((pt2[1] - pt1[1])/(pt2[0] - pt1[0]))
inner_radius = (pt1[0]**2 + y_intersect**2)**0.5
outer_radius = distance + inner_radius

num_gores = 2

angle = (pt1[0])/(inner_radius) * 2 * np.pi / num_gores


angles = np.linspace(0, angle, 100)
plt.plot(np.cos(angles) * inner_radius, np.sin(angles) * inner_radius)

plt.plot(np.cos(angles) * outer_radius, np.sin(angles) * outer_radius)





plt.gca().set_aspect('equal')
plt.grid()
plt.show()