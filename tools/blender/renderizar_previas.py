"""Renderiza uma prévia de cada modelo de assets/blender/modelos.blend (Cycles, CPU).

Uso: python3 tools/blender/renderizar_previas.py <assets> <saida_png>
Gera <saida_png>/<modelo>.png (640×480) e uma folha de contato montada pelo chamador.
"""

import math
import os
import sys

import bpy
from mathutils import Vector


def main():
    assets, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=os.path.join(assets, "blender", "modelos.blend"))
    cena = bpy.context.scene
    cena.render.engine = "CYCLES"
    cena.cycles.device = "CPU"
    cena.cycles.samples = 24
    cena.cycles.use_denoising = False
    cena.render.resolution_x, cena.render.resolution_y = 640, 480
    cena.render.film_transparent = False
    cena.view_settings.exposure = -1.0
    mundo = bpy.data.worlds.new("fundo")
    mundo.color = (0.18, 0.19, 0.21)
    cena.world = mundo
    objs = [o for o in cena.objects if o.type == "MESH"]
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    cena.collection.objects.link(cam)
    cena.camera = cam
    sol = bpy.data.objects.new("sol", bpy.data.lights.new("sol", "SUN"))
    sol.data.energy = 2.0
    sol.rotation_euler = (math.radians(50), math.radians(10), math.radians(35))
    cena.collection.objects.link(sol)
    preench = bpy.data.objects.new("area", bpy.data.lights.new("area", "AREA"))
    preench.data.energy = 80
    preench.data.size = 3
    cena.collection.objects.link(preench)
    for ob in objs:
        for o in objs:
            o.hide_render = o is not ob
        dim = ob.dimensions
        centro = ob.matrix_world @ (sum((Vector(c) for c in ob.bound_box), Vector()) / 8)
        raio = max(dim) * 0.9 + 0.05
        cam.location = centro + Vector((raio * 0.9, raio * 1.6, raio * 0.75))
        cam.rotation_euler = (centro - cam.location).to_track_quat("-Z", "Y").to_euler()
        cam.data.lens = 40
        preench.location = centro + Vector((-raio * 1.5, raio, raio * 1.2))
        preench.rotation_euler = (centro - preench.location).to_track_quat("-Z", "Y").to_euler()
        cena.render.filepath = os.path.join(out, ob.name + ".png")
        bpy.ops.render.render(write_still=True)
        print("prévia:", ob.name)


if __name__ == "__main__":
    main()
