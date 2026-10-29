"""Gera os modelos 3D do Perícia Sim no Blender (módulo bpy) e exporta OBJ/MTL.

Uso (Blender instalado):   blender --background --python tools/blender/gerar_modelos.py -- <saida>
Uso (módulo bpy, pip):     python3 tools/blender/gerar_modelos.py <saida>

Saída: <saida>/models/<nome>.obj + .mtl e <saida>/blender/modelos.blend (fonte editável).

Convenções (as mesmas do jogo):
* metros; origem no centro da base (no piso);
* no Blender, a FRENTE do objeto aponta para +Y e o topo para +Z. A exportação usa
  frente = +Z e topo = Y, de modo que a frente vira +Z no jogo (lado voltado para o
  jogador). Essa conversão espelha o eixo X: (x, y, z) do Blender → (-x, z, y) no jogo;
* o nome do material define a textura do jogo pelo prefixo (madeira, metal, plastico,
  tecido, manta, grade, teclado, etiqueta, papelao, folhagem, concreto, ceramica, vidro,
  tela, livro...). A cor difusa vira tinta; a rugosidade vira o brilho especular;
* UV em metros por projeção de caixa (a textura repete com a escala do jogo); telas
  ("tela") têm UV 0..1 próprio, porque recebem a imagem da área de trabalho/estado;
* orçamento: poucos milhares de triângulos por modelo, para rodar sem GPU dedicada.
"""

import math
import os
import sys

import bpy  # deve vir antes de bmesh/mathutils no módulo pip
import bmesh
from mathutils import Euler, Matrix, Vector

# --------------------------------------------------------------------------- materiais
# nome: (cor RGB 0..1, rugosidade)
MATERIAIS = {
    "madeira": ((0.93, 0.88, 0.80), 0.55),
    "madeira_escura": ((0.62, 0.52, 0.42), 0.6),
    "madeira_clara": ((0.86, 0.78, 0.68), 0.55),
    "metal": ((0.72, 0.74, 0.77), 0.35),
    "metal_claro": ((0.82, 0.84, 0.87), 0.35),
    "metal_escuro": ((0.26, 0.27, 0.30), 0.4),
    "metal_cromado": ((0.80, 0.81, 0.83), 0.15),
    "plastico_escuro": ((0.19, 0.20, 0.23), 0.6),
    "plastico_preto": ((0.10, 0.10, 0.12), 0.55),
    "plastico_cinza": ((0.42, 0.44, 0.48), 0.6),
    "borracha": ((0.07, 0.07, 0.08), 0.9),
    "tecido_azul": ((0.33, 0.38, 0.55), 0.95),
    "tecido_preto": ((0.16, 0.16, 0.18), 0.95),
    "manta_antiestatica": ((1.0, 1.0, 1.0), 0.8),
    "grade": ((1.0, 1.0, 1.0), 0.5),
    "teclado": ((1.0, 1.0, 1.0), 0.6),
    "tela": ((0.05, 0.06, 0.08), 0.1),
    "folhagem": ((0.75, 0.95, 0.70), 0.7),
    "folhagem_escura": ((0.45, 0.65, 0.42), 0.7),
    "ceramica": ((0.72, 0.45, 0.32), 0.7),
    "concreto_terra": ((0.30, 0.22, 0.16), 0.95),
    "livro_vermelho": ((0.58, 0.16, 0.16), 0.8),
    "livro_azul": ((0.16, 0.27, 0.51), 0.8),
    "livro_verde": ((0.24, 0.43, 0.24), 0.8),
    "livro_ocre": ((0.70, 0.59, 0.24), 0.8),
    "livro_roxo": ((0.35, 0.24, 0.43), 0.8),
    "livro_creme": ((0.88, 0.84, 0.74), 0.8),
    "plastico_laranja": ((0.80, 0.28, 0.16), 0.5),
    "plastico_vermelho": ((0.78, 0.20, 0.18), 0.5),
    "plastico_azul": ((0.25, 0.45, 0.80), 0.45),
    "plastico_branco": ((0.90, 0.90, 0.92), 0.5),
    "metal_prata": ((0.70, 0.72, 0.76), 0.3),
    "metal_rosado": ((0.78, 0.66, 0.68), 0.3),
    "metal_dourado": ((0.82, 0.68, 0.30), 0.3),
    "borracha_texturizada": ((0.09, 0.09, 0.10), 0.95),
    "vidro_lente": ((0.10, 0.14, 0.20), 0.05),
    "luz": ((1.0, 0.97, 0.88), 0.5),
    "placa_verde": ((0.08, 0.35, 0.20), 0.6),
    "tecido_cinza": ((0.35, 0.35, 0.37), 0.95),
}


class Modelo:
    """Acumula a geometria de um modelo num único bmesh, com índice de material por face."""

    def __init__(self, nome):
        self.nome = nome
        self.bm = bmesh.new()
        self.mats = []
        self.fix = self.bm.faces.layers.int.new("uv_fixo")
        self.uv = self.bm.loops.layers.uv.new("UVMap")

    def _mat(self, nome):
        if nome not in self.mats:
            self.mats.append(nome)
        return self.mats.index(nome)

    def caixa(self, tam, loc, mat, chanfro=0.0, rot=(0.0, 0.0, 0.0), segs=1):
        """Caixa de dimensões `tam` (x, y, z) centrada em `loc`, opcionalmente chanfrada."""
        res = bmesh.ops.create_cube(self.bm, size=1.0)
        vs = res["verts"]
        m = Matrix.Translation(Vector(loc)) @ Euler(rot).to_matrix().to_4x4() @ Matrix.Diagonal((tam[0], tam[1], tam[2], 1.0))
        bmesh.ops.transform(self.bm, matrix=m, verts=vs)
        faces = {f for v in vs for f in v.link_faces}
        idx = self._mat(mat)
        for f in faces:
            f.material_index = idx
        if chanfro > 0:
            edges = list({e for v in vs for e in v.link_edges})
            lim = min(tam) * 0.45
            res = bmesh.ops.bevel(self.bm, geom=edges, offset=min(chanfro, lim), offset_type="OFFSET", segments=segs, profile=0.5, affect="EDGES", clamp_overlap=True)
            # o chanfro substitui vértices: devolve os da geometria resultante
            for f in res["faces"]:
                f.material_index = idx  # o chanfro cria faces com material 0 por padrão
            todas = [f for f in faces if f.is_valid] + list(res["faces"])
            vs = list({v for f in todas for v in f.verts})
        return vs

    def cilindro(self, r1, r2, alt, loc, mat, segs=16, rot=(0.0, 0.0, 0.0), suave=True):
        """Cone/cilindro com eixo Z (antes da rotação), centrado em `loc`."""
        m = Matrix.Translation(Vector(loc)) @ Euler(rot).to_matrix().to_4x4()
        res = bmesh.ops.create_cone(self.bm, cap_ends=True, cap_tris=False, segments=segs, radius1=r1, radius2=r2, depth=alt, matrix=m)
        idx = self._mat(mat)
        eixo = (Euler(rot).to_matrix() @ Vector((0, 0, 1))).normalized()
        for f in {f for v in res["verts"] for f in v.link_faces}:
            f.material_index = idx
            f.smooth = suave and abs(f.normal.dot(eixo)) < 0.9
        return res["verts"]

    def tela(self, x0, x1, z0, z1, y, mat="tela", inclinacao=0.0, pivo=None):
        """Quadrilátero de tela voltado para +Y, com UV 0..1 (v cresce para cima).
        O u é invertido porque a exportação espelha X: no jogo, u=0 fica à esquerda."""
        cantos = [(x0, z1), (x1, z1), (x1, z0), (x0, z0)]
        uvs = [(1.0, 1.0), (0.0, 1.0), (0.0, 0.0), (1.0, 0.0)]
        vs = []
        for (x, z) in cantos:
            p = Vector((x, y, z))
            if inclinacao and pivo is not None:
                p = Vector(pivo) + Matrix.Rotation(inclinacao, 3, "X") @ (p - Vector(pivo))
            vs.append(self.bm.verts.new(p))
        f = self.bm.faces.new(vs)
        f.material_index = self._mat(mat)
        f[self.fix] = 1
        f.normal_update()
        if f.normal.y < 0:
            f.normal_flip()
            vs = vs[::-1]
            uvs = uvs[::-1]
        for loop in f.loops:
            k = vs.index(loop.vert)
            loop[self.uv].uv = uvs[k]
        return f

    def finalizar(self):
        bm = self.bm
        bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
        bm.normal_update()
        # UV em metros por projeção de caixa (exceto telas)
        for f in bm.faces:
            if f[self.fix]:
                continue
            n = f.normal
            ax = max(range(3), key=lambda i: abs(n[i]))
            for loop in f.loops:
                c = loop.vert.co
                loop[self.uv].uv = (c.x, c.y) if ax == 2 else ((c.y, c.z) if ax == 0 else (c.x, c.z))
        me = bpy.data.meshes.new(self.nome)
        bm.to_mesh(me)
        bm.free()
        for nome in self.mats:
            me.materials.append(material(nome))
        ob = bpy.data.objects.new(self.nome, me)
        return ob


def material(nome):
    if nome in bpy.data.materials:
        return bpy.data.materials[nome]
    cor, rug = MATERIAIS.get(nome, ((0.8, 0.8, 0.8), 0.5))
    mt = bpy.data.materials.new(nome)
    mt.diffuse_color = (*cor, 1.0)
    mt.roughness = rug
    nt = getattr(mt, "node_tree", None)
    if nt is not None:
        p = nt.nodes.get("Principled BSDF")
        if p is not None:
            p.inputs["Base Color"].default_value = (*cor, 1.0)
            p.inputs["Roughness"].default_value = rug
            if nome.startswith("metal"):
                p.inputs["Metallic"].default_value = 0.6
    return mt


# =========================================================================== modelos


def cadeira_escritorio():
    """Cadeira giratória: base estrela de 5 pés com rodízios, pistão, assento, encosto e braços.
    A pessoa senta voltada para -Y (no jogo, de costas para +Z, como a cadeira procedural)."""
    m = Modelo("cadeira_escritorio")
    for k in range(5):
        a = k * math.tau / 5 + math.pi / 2
        d = Vector((math.cos(a), math.sin(a), 0))
        m.caixa((0.29, 0.045, 0.035), Vector((0, 0, 0.075)) + d * 0.15, "plastico_preto", 0.008, rot=(0, 0, a))
        m.cilindro(0.024, 0.024, 0.028, Vector((0, 0, 0.026)) + d * 0.28, "borracha", segs=10, rot=(math.pi / 2, 0, a))
        m.cilindro(0.008, 0.008, 0.03, Vector((0, 0, 0.055)) + d * 0.28, "metal_escuro", segs=6)
    m.cilindro(0.05, 0.045, 0.07, (0, 0, 0.085), "plastico_preto", segs=16)
    m.cilindro(0.028, 0.028, 0.25, (0, 0, 0.24), "metal_cromado", segs=14)
    m.cilindro(0.034, 0.034, 0.08, (0, 0, 0.38), "plastico_preto", segs=14)
    m.caixa((0.30, 0.30, 0.03), (0, 0, 0.425), "plastico_preto", 0.006)
    m.caixa((0.49, 0.47, 0.075), (0, -0.01, 0.475), "tecido_azul", 0.03, segs=3)
    # suporte e encosto (lado +Y)
    m.caixa((0.06, 0.035, 0.30), (0, 0.235, 0.56), "plastico_preto", 0.008, rot=(-0.12, 0, 0))
    m.caixa((0.46, 0.07, 0.53), (0, 0.255, 0.85), "tecido_azul", 0.03, rot=(-0.10, 0, 0), segs=3)
    m.caixa((0.40, 0.02, 0.45), (0, 0.29, 0.85), "plastico_preto", 0.008, rot=(-0.10, 0, 0))
    # braços
    for sx in (-1, 1):
        m.caixa((0.03, 0.05, 0.19), (sx * 0.255, 0.04, 0.585), "plastico_preto", 0.008)
        m.caixa((0.065, 0.25, 0.03), (sx * 0.255, 0.01, 0.685), "plastico_escuro", 0.012)
    return m


def mesa(nome, larg, alt, prof, tampo="madeira", pes="metal_escuro", pes_madeira=False):
    """Mesa de trabalho com tampo chanfrado, pés, travessas laterais e painel de fundo."""
    m = Modelo(nome)
    esp = 0.035
    m.caixa((larg, prof, esp), (0, 0, alt - esp / 2), tampo, 0.006)
    hx, hy = larg / 2 - 0.05, prof / 2 - 0.05
    pe = 0.05 if pes_madeira else 0.04
    for sx in (-1, 1):
        for sy in (-1, 1):
            m.caixa((pe, pe, alt - esp), (sx * hx, sy * hy, (alt - esp) / 2), pes, 0.004)
        m.caixa((pe * 0.7, prof - 0.12, 0.05), (sx * hx, 0, alt - esp - 0.03), pes, 0.003)
        m.caixa((pe * 0.7, prof - 0.12, 0.03), (sx * hx, 0, 0.09), pes, 0.003)
        m.caixa((0.035, 0.035, 0.012), (sx * hx, -hy, 0.006), "borracha")
        m.caixa((0.035, 0.035, 0.012), (sx * hx, hy, 0.006), "borracha")
    # painel de fundo (lado -Y, oposto ao usuário)
    m.caixa((larg - 0.14, 0.015, 0.32), (0, -hy, alt - esp - 0.2), tampo if pes_madeira else pes, 0.003)
    return m


def monitor():
    """Monitor de 24": base, haste e painel fino; a área 'tela' recebe a imagem da sessão."""
    m = Modelo("monitor")
    m.caixa((0.24, 0.16, 0.018), (0, 0, 0.009), "plastico_escuro", 0.006, segs=2)
    m.caixa((0.05, 0.03, 0.25), (0, -0.035, 0.14), "plastico_escuro", 0.008)
    m.cilindro(0.03, 0.03, 0.05, (0, -0.035, 0.33), "plastico_preto", segs=12, rot=(0, math.pi / 2, 0))
    m.caixa((0.62, 0.028, 0.38), (0, -0.01, 0.36), "plastico_preto", 0.006)
    m.caixa((0.30, 0.03, 0.20), (0, -0.03, 0.36), "plastico_escuro", 0.02)  # tampa traseira
    m.caixa((0.04, 0.004, 0.006), (0.24, 0.0045, 0.177), "plastico_cinza")  # logo/botão
    m.tela(-0.295, 0.295, 0.185, 0.535, 0.0042)
    return m


def gabinete_pc():
    """Gabinete torre ATX (0,21 × 0,46 × 0,45): frente com grade, baias e botão; pés de borracha."""
    m = Modelo("gabinete_pc")
    w, d, h = 0.21, 0.46, 0.45
    m.caixa((w, d - 0.02, h - 0.012), (0, -0.01, h / 2 + 0.006), "plastico_escuro", 0.006)
    m.caixa((w, 0.022, h - 0.012), (0, d / 2 - 0.011, h / 2 + 0.006), "plastico_preto", 0.008)
    m.caixa((w * 0.78, 0.004, h * 0.52), (0, d / 2 + 0.001, h * 0.36), "grade")
    for k in range(2):
        m.caixa((w * 0.8, 0.004, 0.034), (0, d / 2 + 0.001, h - 0.07 - k * 0.045), "plastico_cinza", 0.002)
    m.cilindro(0.009, 0.009, 0.006, (0.05, d / 2 + 0.002, h - 0.035), "metal_cromado", segs=12, rot=(math.pi / 2, 0, 0))
    m.caixa((0.03, 0.004, 0.012), (-0.01, d / 2 + 0.001, h - 0.035), "plastico_cinza")  # USB frontais
    # grade traseira e lateral (tampa) com respiro
    m.caixa((0.004, d * 0.5, h * 0.35), (-w / 2 - 0.001, 0.0, h * 0.62), "grade")
    for sx in (-1, 1):
        for sy in (-1, 1):
            m.caixa((0.04, 0.05, 0.012), (sx * (w / 2 - 0.03), sy * (d / 2 - 0.05), 0.006), "borracha", 0.003)
    return m


def notebook():
    """Notebook aberto (0,34 × 0,24): base com teclado e touchpad, dobradiça e tampa inclinada."""
    m = Modelo("notebook")
    m.caixa((0.34, 0.24, 0.018), (0, 0, 0.011), "metal_claro", 0.006, segs=2)
    m.caixa((0.28, 0.11, 0.002), (0, -0.02 + 0.0, 0.0205), "teclado")
    m.caixa((0.10, 0.055, 0.0015), (0, 0.075, 0.0202), "plastico_cinza", 0.004)
    m.cilindro(0.008, 0.008, 0.30, (0, -0.118, 0.021), "metal_escuro", segs=10, rot=(0, math.pi / 2, 0))
    piv = (0, -0.118, 0.021)
    tilt = 0.25
    # tampa: inclinada para trás (-Y) a partir da dobradiça
    lid = Matrix.Translation(Vector(piv)) @ Matrix.Rotation(-tilt, 4, "X") @ Matrix.Translation(Vector((0, -0.004, 0.112)))
    vs = m.caixa((0.34, 0.008, 0.224), (0, 0, 0), "metal_claro", 0.004, segs=2)
    bmesh.ops.transform(m.bm, matrix=lid, verts=vs)
    vs = m.caixa((0.32, 0.002, 0.2), (0, 0, 0), "plastico_preto")
    bmesh.ops.transform(m.bm, matrix=lid @ Matrix.Translation(Vector((0, 0.004, 0.004))), verts=vs)
    m.tela(-0.152, 0.152, piv[2] + 0.02, piv[2] + 0.205, piv[1] + 0.0052, inclinacao=-tilt, pivo=piv)
    return m


def celular():
    """Smartphone (72 × 152 × 8 mm): corpo arredondado, tela, botões laterais e câmera."""
    m = Modelo("celular")
    w, d, h = 0.072, 0.152, 0.008
    m.caixa((w, d, h), (0, 0, h / 2), "metal_escuro", 0.0035, segs=3)
    m.caixa((w - 0.004, d - 0.004, 0.0008), (0, 0, h - 0.0002), "plastico_preto", 0.002)
    f = m.tela(-w * 0.46, w * 0.46, -d * 0.47, d * 0.47, 0.0, mat="tela")
    # tela no plano XY (topo): gira o quad para ficar deitado, voltado para +Z
    for v in f.verts:
        x, y, z = v.co.x, v.co.y, v.co.z
        v.co = Vector((x, -z, h + 0.0003))
    f.normal_update()
    if f.normal.z < 0:
        f.normal_flip()
    m.caixa((0.0016, 0.022, 0.003), (w / 2 + 0.0004, 0.02, h / 2), "metal_escuro", 0.0006)
    m.caixa((0.0016, 0.012, 0.003), (w / 2 + 0.0004, -0.012, h / 2), "metal_escuro", 0.0006)
    m.cilindro(0.009, 0.009, 0.0016, (-0.018, -0.055, -0.0006), "plastico_preto", segs=14)
    return m


def bancada_lab():
    """Bancada pericial (2,64 × 1,14 × 0,905): tampo antiestático, borda metálica e dois
    gaveteiros com três gavetas, puxadores e rodapé."""
    m = Modelo("bancada_lab")
    m.caixa((2.6, 1.1, 0.04), (0, 0, 0.885), "manta_antiestatica", 0.004)
    m.caixa((2.64, 1.14, 0.05), (0, 0, 0.84), "metal", 0.006)
    for sx in (-1, 1):
        cx = sx * 0.95
        m.caixa((0.6, 1.0, 0.76), (cx, 0, 0.43), "metal_claro", 0.006)
        m.caixa((0.6, 0.96, 0.05), (cx, 0, 0.025), "metal_escuro", 0.004)
        for k in range(3):
            z = 0.15 + k * 0.24
            m.caixa((0.56, 0.012, 0.215), (cx, 0.505, z + 0.02), "metal_claro", 0.004)
            m.caixa((0.20, 0.02, 0.022), (cx, 0.522, z + 0.08), "metal_cromado", 0.006)
            m.caixa((0.08, 0.002, 0.03), (cx - 0.2, 0.512, z + 0.08), "etiqueta")
    # travessa e painel de fundo entre os gaveteiros
    m.caixa((1.3, 0.02, 0.5), (0, -0.5, 0.55), "metal", 0.004)
    m.caixa((1.3, 0.04, 0.04), (0, -0.4, 0.12), "metal_escuro", 0.004)
    return m


def armario_aco():
    """Armário de aço para evidências (1,0 × 0,6 × 2,0): duas portas, puxadores, respiros e base."""
    m = Modelo("armario_aco")
    w, d, h = 1.0, 0.6, 2.0
    m.caixa((w, d, h - 0.06), (0, 0, h / 2 + 0.03), "metal", 0.008)
    m.caixa((w - 0.02, d - 0.04, 0.06), (0, 0, 0.03), "metal_escuro", 0.004)
    for sx in (-1, 1):
        m.caixa((w / 2 - 0.02, 0.014, h - 0.12), (sx * (w / 4 + 0.002), d / 2 + 0.006, h / 2 + 0.03), "metal_claro", 0.006)
        m.caixa((0.022, 0.03, 0.2), (sx * 0.06, d / 2 + 0.025, 1.05), "metal_cromado", 0.008)
        for k in range(5):
            m.caixa((0.16, 0.004, 0.012), (sx * 0.27, d / 2 + 0.014, h - 0.22 - k * 0.03), "metal_escuro")
            m.caixa((0.16, 0.004, 0.012), (sx * 0.27, d / 2 + 0.014, 0.28 + k * 0.03), "metal_escuro")
    m.cilindro(0.012, 0.012, 0.012, (0.0, d / 2 + 0.018, 1.2), "metal_cromado", segs=12, rot=(math.pi / 2, 0, 0))
    return m


def estante_aco():
    """Estante de aço (1,4 × 0,4 × 2,0): cinco prateleiras (alturas iguais às do jogo),
    colunas em L e travamento em X no fundo."""
    m = Modelo("estante_aco")
    L, P, H = 1.4, 0.4, 2.0
    for k in range(5):
        z = 0.1 + k * 0.45
        m.caixa((L - 0.02, P - 0.02, 0.02), (0, 0, z), "metal", 0.003)
        m.caixa((L - 0.02, 0.012, 0.035), (0, P / 2 - 0.016, z - 0.01), "metal_claro", 0.002)
    for sx in (-1, 1):
        for sy in (-1, 1):
            x, y = sx * (L / 2 - 0.02), sy * (P / 2 - 0.02)
            m.caixa((0.04, 0.004, H), (x, y + sy * 0.018, H / 2), "metal_escuro")
            m.caixa((0.004, 0.04, H), (x + sx * 0.018, y, H / 2), "metal_escuro")
    diag = math.atan2(1.8, L - 0.08)
    comp = math.hypot(1.8, L - 0.08)
    for s in (-1, 1):
        m.caixa((comp, 0.006, 0.02), (0, -P / 2 + 0.01, 1.0), "metal_escuro", rot=(0, s * diag, 0))
    return m


def estante_livros():
    """Estante de madeira (1,3 × 0,35 × 1,9) com fundo e três prateleiras de livros; a quarta
    prateleira fica livre (o roteador do caso é desenhado pelo jogo)."""
    m = Modelo("estante_livros")
    L, P, H = 1.3, 0.35, 1.9
    for k in range(5):
        m.caixa((L - 0.06, P, 0.03), (0, 0, 0.05 + k * 0.45), "madeira", 0.003)
    for sx in (-1, 1):
        m.caixa((0.03, P, H), (sx * (L / 2 - 0.015), 0, H / 2), "madeira", 0.003)
    m.caixa((L - 0.04, 0.01, H - 0.04), (0, -P / 2 + 0.005, H / 2), "madeira_escura")
    m.caixa((L - 0.06, 0.02, 0.06), (0, P / 2 - 0.01, 0.03), "madeira", 0.003)
    cores = ["livro_vermelho", "livro_azul", "livro_verde", "livro_ocre", "livro_roxo", "livro_creme"]
    for k in range(3):
        x = -L / 2 + 0.06
        b = 0
        while x < L / 2 - 0.12:
            esp = 0.025 + ((b * 7 + k * 3) % 4) * 0.008
            alt = 0.20 + ((b * 5 + k) % 5) * 0.022
            prof = 0.21 + ((b * 3) % 3) * 0.02
            z0 = 0.05 + k * 0.45 + 0.015
            inclina = 0.12 if (b + k) % 11 == 6 else 0.0
            cor = cores[(b * 3 + k) % len(cores)]
            m.caixa((esp, prof, alt), (x + esp / 2, 0.02, z0 + alt / 2), cor, rot=(0, inclina, 0))
            m.caixa((esp + 0.001, 0.004, 0.025), (x + esp / 2, 0.02 + prof / 2 + 0.001, z0 + alt * 0.75), "livro_creme", rot=(0, inclina, 0))
            x += esp + 0.004 + (0.02 if inclina else 0.0)
            b += 1
    return m


def gaveteiro():
    """Gaveteiro de madeira (0,45 × 0,55 × 0,70) com três gavetas; a de cima está aberta."""
    m = Modelo("gaveteiro")
    w, d, h = 0.45, 0.55, 0.7
    m.caixa((w, d, 0.03), (0, 0, h - 0.015), "madeira", 0.004)
    for sx in (-1, 1):
        m.caixa((0.02, d, h - 0.03), (sx * (w / 2 - 0.01), 0, (h - 0.03) / 2), "madeira_clara", 0.003)
    m.caixa((w - 0.04, 0.015, h - 0.03), (0, -d / 2 + 0.0075, (h - 0.03) / 2), "madeira_clara")
    for k, z in enumerate((0.48, 0.28, 0.08)):
        aberto = 0.12 if k == 0 else 0.0
        hz = 0.17
        m.caixa((w - 0.05, 0.018, hz), (0, d / 2 - 0.009 + aberto, z + hz / 2 - 0.02), "madeira", 0.004)
        m.caixa((0.12, 0.022, 0.018), (0, d / 2 + 0.012 + aberto, z + hz / 2 + 0.02), "metal_cromado", 0.006)
        if aberto:
            m.caixa((w - 0.07, d * 0.6, 0.012), (0, d / 2 - d * 0.3 + aberto, z - 0.01), "madeira_clara")
            for sx in (-1, 1):
                m.caixa((0.012, d * 0.6, 0.11), (sx * (w / 2 - 0.045), d / 2 - d * 0.3 + aberto, z + 0.045), "madeira_clara")
    return m


def planta():
    """Planta em vaso de cerâmica (≈ 0,95 m): vaso cônico, terra e folhas longas arqueadas."""
    m = Modelo("planta")
    m.cilindro(0.20, 0.16, 0.40, (0, 0, 0.2), "ceramica", segs=20)
    m.cilindro(0.205, 0.205, 0.03, (0, 0, 0.395), "ceramica", segs=20)
    m.cilindro(0.185, 0.185, 0.01, (0, 0, 0.385), "concreto_terra", segs=20, suave=False)
    for k in range(14):
        a = k * 2.399  # ângulo áureo
        rr = 0.03 + (k % 3) * 0.025
        alt = 0.32 + (k % 4) * 0.07
        inc = 0.25 + (k % 3) * 0.12
        mat = "folhagem" if k % 2 else "folhagem_escura"
        base = Vector((math.cos(a) * rr, math.sin(a) * rr, 0.39))
        # folha em dois segmentos (curva)
        for seg, (f0, ang) in enumerate(((0.0, inc), (0.5, inc * 2.1))):
            comp = alt * 0.55
            c = base + Vector((math.cos(a) * math.sin(inc) * alt * f0, math.sin(a) * math.sin(inc) * alt * f0, math.cos(inc) * alt * f0 * 0.95))
            dirv = Vector((math.cos(a) * math.sin(ang), math.sin(a) * math.sin(ang), math.cos(ang)))
            centro = c + dirv * comp / 2
            rot = Vector((0, 0, 1)).rotation_difference(dirv).to_euler()
            larg = 0.055 if seg == 0 else 0.04
            vs = m.caixa((larg, 0.006, comp), (0, 0, 0), mat, rot=(0, 0, 0))
            mtx = Matrix.Translation(centro) @ rot.to_matrix().to_4x4() @ Matrix.Rotation(a + math.pi / 2, 4, "Z")
            bmesh.ops.transform(m.bm, matrix=mtx, verts=vs)
    return m


# =========================================================================== ferramentas de trabalho


def camera_foto():
    """Câmera fotográfica DSLR (≈ 14 × 10 × 11 cm): corpo com empunhadura, prisma, objetiva
    voltada para a frente, botões e visor traseiro."""
    m = Modelo("camera_foto")
    m.caixa((0.14, 0.07, 0.09), (0, 0, 0.05), "borracha_texturizada", 0.012, segs=2)
    m.caixa((0.04, 0.085, 0.095), (0.055, 0.008, 0.052), "borracha_texturizada", 0.015, segs=2)  # empunhadura
    m.caixa((0.055, 0.055, 0.03), (-0.01, -0.005, 0.105), "plastico_preto", 0.008)  # prisma
    m.caixa((0.035, 0.02, 0.012), (-0.01, -0.035, 0.112), "plastico_preto", 0.004)  # ocular
    m.cilindro(0.036, 0.036, 0.05, (-0.008, 0.06, 0.05), "plastico_preto", segs=20, rot=(math.pi / 2, 0, 0))
    m.cilindro(0.038, 0.038, 0.022, (-0.008, 0.075, 0.05), "borracha", segs=20, rot=(math.pi / 2, 0, 0))  # anel de foco
    m.cilindro(0.03, 0.03, 0.004, (-0.008, 0.0875, 0.05), "vidro_lente", segs=20, rot=(math.pi / 2, 0, 0))
    m.cilindro(0.007, 0.007, 0.006, (0.055, 0.02, 0.103), "metal_cromado", segs=10)  # disparador
    m.cilindro(0.014, 0.014, 0.008, (-0.045, -0.005, 0.098), "plastico_escuro", segs=14)  # seletor de modos
    m.caixa((0.075, 0.003, 0.05), (-0.012, -0.0365, 0.05), "tela")
    for sx in (-1, 1):
        m.caixa((0.008, 0.012, 0.014), (sx * 0.072, -0.01, 0.085), "metal_escuro", 0.002)  # alças
    return m


def bloqueador_escrita():
    """Bloqueador de escrita (22 × 14 × 7 cm): carcaça laranja, portas de origem e destino
    atrás, cabo SATA e etiqueta. Os LEDs são desenhados pelo jogo (indicam o estado)."""
    m = Modelo("bloqueador_escrita")
    m.caixa((0.22, 0.14, 0.064), (0, 0, 0.032), "plastico_laranja", 0.012, segs=2)
    m.caixa((0.21, 0.13, 0.008), (0, 0, 0.066), "plastico_escuro", 0.004)
    m.caixa((0.08, 0.05, 0.002), (-0.04, 0.0, 0.0705), "etiqueta")
    for k, x in enumerate((-0.07, -0.02, 0.04)):
        m.caixa((0.03 if k < 2 else 0.018, 0.006, 0.012), (x, -0.072, 0.032), "plastico_preto", 0.002)
    m.cilindro(0.006, 0.006, 0.008, (0.08, -0.073, 0.032), "metal_cromado", segs=10, rot=(math.pi / 2, 0, 0))
    # cabo SATA saindo para trás
    m.caixa((0.012, 0.09, 0.004), (-0.07, -0.12, 0.012), "plastico_vermelho", 0.001, rot=(0.25, 0, 0.1))
    for sx in (-1, 1):
        for sy in (-1, 1):
            m.cilindro(0.008, 0.008, 0.004, (sx * 0.09, sy * 0.055, 0.002), "borracha", segs=8)
    return m


def kit_embalagem():
    """Material de acondicionamento: Faraday bag (bolsa metalizada dobrada com velcro),
    saco antiestático, rolo de fita de lacre e bloco de lacres numerados."""
    m = Modelo("kit_embalagem")
    # Faraday bag
    m.caixa((0.24, 0.17, 0.012), (-0.12, 0.0, 0.006), "metal_prata", 0.004, rot=(0, 0, -0.2))
    m.caixa((0.22, 0.05, 0.004), (-0.135, 0.055, 0.014), "tecido_cinza", 0.001, rot=(0, 0, -0.2))  # aba com velcro
    # saco antiestático
    m.caixa((0.2, 0.14, 0.008), (0.15, -0.02, 0.004), "metal_rosado", 0.002, rot=(0, 0, 0.3))
    m.caixa((0.19, 0.006, 0.0015), (0.13, 0.026, 0.0085), "plastico_branco", rot=(0, 0, 0.3))  # fecho zip
    # rolo de fita de lacre
    m.cilindro(0.045, 0.045, 0.05, (0.36, 0.03, 0.025), "plastico_vermelho", segs=20)
    m.cilindro(0.021, 0.021, 0.052, (0.36, 0.03, 0.025), "plastico_branco", segs=14)
    # bloco de lacres numerados
    m.caixa((0.07, 0.05, 0.012), (0.33, -0.07, 0.006), "etiqueta", 0.002)
    return m


def luminaria_bancada():
    """Luminária articulada de bancada: base pesada, coluna, braço e cabeça com difusor."""
    m = Modelo("luminaria_bancada")
    m.cilindro(0.07, 0.075, 0.025, (0, 0, 0.0125), "metal_escuro", segs=20)
    m.cilindro(0.012, 0.012, 0.45, (0, 0, 0.25), "metal_escuro", segs=10)
    m.cilindro(0.02, 0.02, 0.03, (0, 0, 0.47), "plastico_preto", segs=12, rot=(0, math.pi / 2, 0))
    # braço até a cabeça (no jogo: 0,3 m para -X e 0,15 m para a frente; no Blender X é espelhado)
    ang = math.atan2(0.15, 0.3)
    m.caixa((0.34, 0.016, 0.016), (0.15, 0.075, 0.47), "metal_escuro", 0.004, rot=(0, 0, ang))
    m.caixa((0.15, 0.1, 0.06), (0.3, 0.15, 0.43), "metal_escuro", 0.01, rot=(0, 0, ang))
    m.caixa((0.13, 0.08, 0.004), (0.3, 0.15, 0.398), "luz", rot=(0, 0, ang))
    return m


def teclado():
    """Teclado ABNT2 (44 × 14 cm): carcaça com leve inclinação; teclas vêm da textura."""
    m = Modelo("teclado")
    m.caixa((0.44, 0.14, 0.018), (0, 0, 0.011), "plastico_preto", 0.005, rot=(0.05, 0, 0))
    m.caixa((0.42, 0.12, 0.004), (0, -0.002, 0.021), "teclado", rot=(0.05, 0, 0))
    for sx in (-1, 1):
        m.caixa((0.03, 0.012, 0.006), (sx * 0.19, -0.06, 0.003), "borracha")
    return m


def mouse():
    """Mouse óptico: corpo arredondado em duas metades e roda."""
    m = Modelo("mouse")
    m.caixa((0.06, 0.105, 0.03), (0, 0, 0.016), "plastico_preto", 0.013, segs=3)
    m.caixa((0.002, 0.04, 0.002), (0, 0.03, 0.0315), "plastico_escuro")
    m.cilindro(0.007, 0.007, 0.006, (0, 0.028, 0.032), "borracha", segs=10, rot=(0, math.pi / 2, 0))
    return m


def hd_35():
    """Disco rígido 3,5" (102 × 147 × 26 mm): tampa com etiqueta, base fundida, placa
    lógica, conectores SATA (atrás) e parafusos."""
    m = Modelo("hd_35")
    w, d, h = 0.102, 0.147, 0.026
    m.caixa((w, d, h - 0.003), (0, 0, (h - 0.003) / 2 + 0.002), "metal_prata", 0.002)
    m.caixa((w - 0.004, d - 0.004, 0.002), (0, 0, 0.001), "placa_verde")
    m.caixa((0.085, 0.095, 0.0006), (0, 0.012, h - 0.0007), "etiqueta")
    m.caixa((0.07, 0.006, 0.012), (0, -d / 2 - 0.002, 0.008), "plastico_preto")
    for x in (-0.04, 0.04):
        for y in (-0.06, 0.06):
            m.cilindro(0.003, 0.003, 0.001, (x, y, h - 0.0005), "metal_escuro", segs=8)
    return m


def ssd_25():
    """SSD 2,5" (70 × 100 × 7 mm) com etiqueta e conector SATA."""
    m = Modelo("ssd_25")
    m.caixa((0.07, 0.1, 0.007), (0, 0, 0.0035), "metal_escuro", 0.0015)
    m.caixa((0.06, 0.07, 0.0005), (0, 0.008, 0.0072), "etiqueta")
    m.caixa((0.04, 0.004, 0.005), (0, -0.0515, 0.0035), "plastico_preto")
    return m


def pendrive():
    """Pendrive (55 × 20 × 12 mm): corpo plástico, conector USB metálico e argola."""
    m = Modelo("pendrive")
    m.caixa((0.046, 0.02, 0.012), (-0.004, 0, 0.006), "plastico_azul", 0.004, segs=2)
    m.caixa((0.018, 0.0125, 0.0045), (-0.036, 0, 0.006), "metal_cromado", 0.0005)  # no jogo, +X
    m.cilindro(0.005, 0.005, 0.003, (0.022, 0, 0.006), "metal_escuro", segs=10, rot=(math.pi / 2, 0, 0))
    return m


def tablet_perito():
    """Tablet robusto do perito (26 × 18 cm): moldura com cantoneiras de borracha laranja,
    tela, câmera, alça traseira e etiqueta "PERÍCIA". Deitado: tela para cima (+Z),
    parte superior da tela voltada para -Y (no jogo, para longe do perito ao segurar)."""
    m = Modelo("tablet_perito")
    w, d, h = 0.26, 0.18, 0.014
    m.caixa((w, d, h), (0, 0, h / 2), "plastico_escuro", 0.004, segs=2)
    for sx in (-1, 1):
        for sy in (-1, 1):
            m.caixa((0.04, 0.04, h + 0.004), (sx * (w / 2 - 0.016), sy * (d / 2 - 0.016), h / 2), "plastico_laranja", 0.008, segs=2)
    m.caixa((w - 0.03, d - 0.03, 0.0008), (0, 0, h + 0.0002), "plastico_preto", 0.002)
    f = m.tela(-(w - 0.05) / 2, (w - 0.05) / 2, -(d - 0.05) / 2, (d - 0.05) / 2, 0.0, mat="tela")
    for v in f.verts:
        x, z = v.co.x, v.co.z
        v.co = Vector((x, -z, h + 0.0008))
    f.normal_update()
    if f.normal.z < 0:
        f.normal_flip()
    m.cilindro(0.003, 0.003, 0.001, (0, -d / 2 + 0.012, h + 0.0006), "vidro_lente", segs=8)
    m.caixa((0.16, 0.035, 0.006), (0, 0.0, -0.002), "tecido_cinza", 0.002)  # alça de mão no verso
    m.caixa((0.07, 0.02, 0.0006), (0.06, 0.05, -0.0003), "etiqueta")
    m.caixa((0.003, 0.03, 0.005), (w / 2 + 0.0005, -0.03, h / 2), "metal_escuro", 0.001)  # botões
    return m


MODELOS = [
    cadeira_escritorio,
    lambda: mesa("mesa_escritorio", 1.8, 0.76, 0.8, tampo="madeira", pes="madeira_escura", pes_madeira=True),
    lambda: mesa("mesa_estacao", 1.5, 0.75, 0.75, tampo="madeira_clara", pes="metal_escuro"),
    monitor,
    gabinete_pc,
    notebook,
    celular,
    bancada_lab,
    armario_aco,
    estante_aco,
    estante_livros,
    gaveteiro,
    planta,
    camera_foto,
    bloqueador_escrita,
    kit_embalagem,
    luminaria_bancada,
    teclado,
    mouse,
    hd_35,
    ssd_25,
    pendrive,
    tablet_perito,
]


def main():
    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    saida = os.path.abspath(args[0] if args else "assets")
    os.makedirs(os.path.join(saida, "models"), exist_ok=True)
    os.makedirs(os.path.join(saida, "blender"), exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    cena = bpy.context.scene
    objs = []
    for k, fn in enumerate(MODELOS):
        mod = fn()
        ob = mod.finalizar()
        col = bpy.data.collections.new(ob.name)
        cena.collection.children.link(col)
        col.objects.link(ob)
        objs.append(ob)
    for ob in objs:
        for o in bpy.context.view_layer.objects:
            o.select_set(False)
        ob.select_set(True)
        bpy.context.view_layer.objects.active = ob
        caminho = os.path.join(saida, "models", ob.name + ".obj")
        bpy.ops.wm.obj_export(
            filepath=caminho,
            export_selected_objects=True,
            forward_axis="Z",
            up_axis="Y",
            export_materials=True,
            export_uv=True,
            export_normals=True,
            export_triangulated_mesh=True,
            apply_modifiers=True,
            path_mode="STRIP",
        )
        tri = sum(len(p.vertices) - 2 for p in ob.data.polygons)
        print("modelo %-20s %6d triângulos  %s" % (ob.name, tri, tuple(round(v, 3) for v in ob.dimensions)))
    # dispõe os objetos lado a lado no arquivo-fonte para facilitar a edição
    x = 0.0
    for ob in objs:
        ob.location.x = x + ob.dimensions.x / 2
        x += ob.dimensions.x + 0.5
    bpy.context.preferences.filepaths.save_version = 0  # sem cópias .blend1
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(saida, "blender", "modelos.blend"))
    print("ok:", saida)


if __name__ == "__main__":
    main()
