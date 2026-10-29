"""Renderiza os ícones das ferramentas (PNG 128×128, fundo transparente) para a maleta do jogo.

Uso: python3 tools/blender/renderizar_icones.py <assets>
Lê <assets>/blender/modelos.blend e grava <assets>/icons/<modelo>.png.
"""

import math
import os
import sys

import bpy
from mathutils import Vector

ICONES = [
    "camera_foto", "oficio_prancheta", "tesoura", "chave_phillips", "lanterna", "placa_vestigio", "cabo_sata", "adaptador_m2",
    "leitor_sd", "cabo_usb", "cabo_branco", "bateria_externa", "faraday_bag", "papel_aluminio", "ejetor_sim", "pendrive_coleta",
    "saco_evidencia", "fita_lacre", "etiquetas_caneta", "caixa_luvas", "pulseira_esd", "maleta_pericial", "tablet_perito",
]


def main():
    assets = sys.argv[1]
    out = os.path.join(assets, "icons")
    os.makedirs(out, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=os.path.join(assets, "blender", "modelos.blend"))
    cena = bpy.context.scene
    cena.render.engine = "CYCLES"
    cena.cycles.device = "CPU"
    cena.cycles.samples = 16
    cena.cycles.use_denoising = False
    cena.render.resolution_x = cena.render.resolution_y = 128
    cena.render.film_transparent = True
    cena.view_settings.exposure = -0.6
    mundo = bpy.data.worlds.new("fundo")
    mundo.color = (0.5, 0.5, 0.55)
    cena.world = mundo
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    cam.data.type = "ORTHO"
    cena.collection.objects.link(cam)
    cena.camera = cam
    sol = bpy.data.objects.new("sol", bpy.data.lights.new("sol", "SUN"))
    sol.data.energy = 3.0
    sol.rotation_euler = (math.radians(40), math.radians(15), math.radians(30))
    cena.collection.objects.link(sol)
    for nome in ICONES:
        ob = bpy.data.objects.get(nome)
        if ob is None:
            print("sem modelo:", nome)
            continue
        for o in cena.objects:
            if o.type == "MESH":
                o.hide_render = o is not ob
        cs = [ob.matrix_world @ Vector(c) for c in ob.bound_box]
        centro = sum(cs, Vector()) / 8
        dim = max(ob.dimensions)
        d = Vector((0.55, -0.9, 0.75)).normalized()
        cam.location = centro + d * (dim * 3 + 0.2)
        cam.rotation_euler = (centro - cam.location).to_track_quat("-Z", "Y").to_euler()
        cam.data.ortho_scale = dim * 1.15
        cena.render.filepath = os.path.join(out, nome + ".png")
        bpy.ops.render.render(write_still=True)
        print("ícone:", nome)


if __name__ == "__main__":
    main()
