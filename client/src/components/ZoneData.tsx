import {
  Match,
  Switch,
  createEffect,
  createMemo,
  createResource,
  createSignal,
  on,
  onCleanup,
  onMount,
} from "solid-js";
import * as THREE from "three";
import { useParams } from "@solidjs/router";
import { getZoneModel } from "../custom_bindings";
import { writeText, readText } from '@tauri-apps/plugin-clipboard-manager';
import { acceleratedRaycast } from "three-mesh-bvh";
import CameraControls from "camera-controls";
import {
  CSS2DObject,
  CSS2DRenderer,
} from "three/examples/jsm/renderers/CSS2DRenderer.js";
import { FlyControls, MapControls } from "three/examples/jsm/Addons.js";
import Stats from 'three/addons/libs/stats.module.js';
import { unwrap } from "../util";
import { setupBaseScene } from "../graphics/scene";
import { cleanupNode, parseCoordinatesToVector3, roundDecimals } from "../graphics/util";
import { addMapControls, adjustCameraAspect } from "../graphics/camera";
import { ColorKind, colorMesh, createZoneMesh, getHitData, getMapId, markLineCollisions, prepareMeshData } from "../graphics/ximesh";
import { TargetInfo, ZoneInfoBox } from "./ZoneInfoBox";

// Add the extension functions
THREE.Mesh.prototype.raycast = acceleratedRaycast;

interface ZoneDataProps { }

function ZoneData({ }: ZoneDataProps) {
  const params = useParams();
  const zoneId = parseInt(params.id);

  const [xiMeshBuffer] = createResource(
    async () => {
      const meshData = unwrap(await getZoneModel(zoneId));
      return meshData;
    },
    { initialValue: undefined }
  );

  const preparedData = createMemo(() => {
    const buffer = xiMeshBuffer();
    if (!buffer) {
      return;
    }
    return prepareMeshData(buffer);
  });

  const scene = createMemo(() => {
    return setupBaseScene();
  })

  const camera = createMemo(() => {
    const camera = new THREE.PerspectiveCamera(60, 1, 0.1, 5000);
    camera.position.set(0, 500, 0);
    camera.lookAt(0, 0, 0);
    return camera;
  })

  const raycaster = new THREE.Raycaster();
  raycaster.firstHitOnly = true;

  const clock = new THREE.Clock();
  const stats = new Stats();
  stats.dom.style.position = "absolute";

  const cameraMouse = new THREE.Vector2(0, 0);
  const screenMouse = new THREE.Vector2(0, 0);

  let controls: CameraControls | FlyControls | MapControls;
  let canvasElement: HTMLCanvasElement = undefined!;
  let labelRendererElement: HTMLDivElement = undefined!;
  let coordLabelRef: HTMLDivElement = undefined!;
  let coordsNeedUpdate = false;

  const [getNeedsResize, setNeedsResize] = createSignal<boolean>(true);
  function resizeCanvas() {
    const parentRect = canvasElement.parentElement!.getBoundingClientRect();
    canvasElement.width = parentRect.width;
    canvasElement.height = parentRect.height;
    setNeedsResize(true);
  }

  const [getColorKind, setColorKind] = createSignal<ColorKind>(ColorKind.Materials);
  const [getStartPos, setStartPos] = createSignal<THREE.Vector3 | undefined>();
  const [getEndPos, setEndPos] = createSignal<THREE.Vector3 | undefined>();

  const [getTargetInfo, setTargetInfo] = createSignal<TargetInfo | undefined>();

  onMount(() => {
    window.addEventListener("resize", resizeCanvas);

    canvasElement.addEventListener("mousemove", event => {
      const canvas = canvasElement;
      cameraMouse.x = (2 * event.offsetX) / canvas.offsetWidth - 1;
      cameraMouse.y = (-2 * event.offsetY) / canvas.offsetHeight + 1;
      screenMouse.x = event.offsetX;
      screenMouse.y = event.offsetY;
      coordsNeedUpdate = true;
    });

    canvasElement.addEventListener("mouseout", _event => {
      coordLabelRef.style.display = "none";
      coordsNeedUpdate = false;
    });

    canvasElement.addEventListener("click", event => {
      if (!event.ctrlKey && !event.shiftKey) {
        return;
      }

      raycaster.firstHitOnly = false;
      const hits = castRayOntoMesh();
      raycaster.firstHitOnly = true;

      if (!hits || hits.length == 0) {
        return;
      }

      const hitsData = getHitData(zoneMesh()!, xiMeshBuffer(), preparedData()!, hits);

      for (let i = 0; i < hitsData.length; i++) {
        const hitData = hitsData[i];
        console.log(`======================= Hit ${i} =======================`)
        console.log("Hit", hitData.hit);
        console.log("Block", hitData.block);
        console.log("Placement", hitData.placement);
        console.log("Material", hitData.material);
      }

      const firstHit = hits[0];
      const markerPos = new THREE.Vector3(roundDecimals(firstHit.x, 3), roundDecimals(-firstHit.y - 2.25, 3), roundDecimals(-firstHit.z, 3));

      if (event.ctrlKey) {
        setStartPos(markerPos);
      } else if (event.shiftKey) {
        setEndPos(markerPos);
      }
    });

    controls = addMapControls(camera(), canvasElement);

    const renderer = new THREE.WebGLRenderer({ canvas: canvasElement, antialias: true, alpha: true });
    const labelRenderer = new CSS2DRenderer({ element: labelRendererElement });

    renderer.setAnimationLoop(() => animate(renderer, labelRenderer));
  });

  function animate(renderer: THREE.WebGLRenderer, labelRenderer: CSS2DRenderer) {
    stats.update();

    const delta = clock.getDelta();
    controls?.update(delta);

    if (getNeedsResize()) {
      const canvas = canvasElement;
      const width = canvas.clientWidth;
      const height = canvas.clientHeight;
      renderer.setSize(width, height, false);
      labelRenderer.setSize(width, height);
      adjustCameraAspect(camera(), canvasElement);
      setNeedsResize(false);
    }

    if (coordsNeedUpdate) {
      coordsNeedUpdate = false;
      updatePosLabel();
    }

    renderer.render(scene(), camera());
    labelRenderer.render(scene(), camera());
  }

  function updatePosLabel() {
    const hits = castRayOntoMesh();
    if (hits) {
      const hitsData = getHitData(zoneMesh()!, xiMeshBuffer(), preparedData()!, hits);
      const hitData = hitsData[0];
      setTargetInfo({
        x: hitData.hit.x,
        y: hitData.hit.y * -1,
        z: hitData.hit.z * -1,
        mapId: getMapId(hitData.placement),
        material: hitData.material,
        cellIdx: hitData.cellIdx,
        cellEntryIdx: hitData.entryIdx,
        block: hitData.block,
        placement: hitData.placement,
      });

      coordLabelRef.textContent = `${hitData.hit.x.toFixed(1)}, ${(hitData.hit.y * -1).toFixed(1)}, ${(hitData.hit.z * -1).toFixed(1)}`;
      coordLabelRef.style.left = `${screenMouse.x + 20}px`;
      coordLabelRef.style.top = `${screenMouse.y - 20}px`;
      coordLabelRef.style.display = "block";
    } else {
      coordLabelRef.style.display = "none";
      setTargetInfo(undefined);
    }
  }

  onCleanup(() => {
    window.removeEventListener("resize", resizeCanvas);
    cleanupNode(scene());
    if (controls) {
      controls.dispose();
    }
    scene().clear();
    camera().clear();
  });

  // Zone model mesh
  const zoneMesh = createMemo(on(xiMeshBuffer, (buffer?: ArrayBufferLike, _prevBuffer?: ArrayBufferLike, prevMesh?: THREE.Mesh) => {
    if (prevMesh) {
      console.log("Cleaning up previous mesh");
      scene().remove(prevMesh);
      cleanupNode(prevMesh);
    }

    if (!buffer) {
      return;
    }

    const mesh = createZoneMesh(zoneId, buffer, preparedData()!, getColorKind());
    scene().add(mesh);

    onCleanup(() => {
      scene().remove(mesh);
      cleanupNode(mesh);
    });

    return mesh;
  }), undefined);

  // Start marker mesh
  createEffect(on(getStartPos, (pos?: THREE.Vector3, _prevPos?: THREE.Vector3, prevMesh?: THREE.Mesh) => {
    if (!pos) {
      if (prevMesh) {
        onCleanup(() => {
          scene().remove(prevMesh);
          cleanupNode(prevMesh);
        });
      }
      return;
    }

    let mesh = prevMesh;
    if (!mesh) {
      const geo = new THREE.SphereGeometry(0.5);
      const mat = new THREE.MeshPhongMaterial({
        color: new THREE.Color(0, 1, 0),
      });
      mesh = new THREE.Mesh(geo, mat);
      mesh.visible = true;
      scene().add(mesh);
    }

    mesh.position.copy(pos);

    return mesh;
  }));

  // End marker mesh
  createEffect(on(getEndPos, (pos?: THREE.Vector3, _prevPos?: THREE.Vector3, prevMesh?: THREE.Mesh) => {
    if (!pos) {
      if (prevMesh) {
        onCleanup(() => {
          scene().remove(prevMesh);
          cleanupNode(prevMesh);
        });
      }
      return;
    }

    let mesh = prevMesh;
    if (!mesh) {
      const geo = new THREE.SphereGeometry(0.5);
      const mat = new THREE.MeshPhongMaterial({
        color: new THREE.Color(0, 0, 1),
      });
      mesh = new THREE.Mesh(geo, mat);
      scene().add(mesh);
    }

    mesh.position.copy(pos);

    return mesh;
  }));

  // Line between markers
  createEffect(on([getStartPos, getEndPos], (
    value: [THREE.Vector3 | undefined, THREE.Vector3 | undefined],
    _prevValue?: [THREE.Vector3 | undefined, THREE.Vector3 | undefined],
    prevLine?: THREE.Line) => {
    if (!value[0] || !value[1]) {
      if (prevLine) {
        onCleanup(() => {
          scene().remove(prevLine);
          cleanupNode(prevLine);
        });
      }
      return;
    }

    const mesh = zoneMesh();
    if (!mesh) {
      return;
    }

    let line = prevLine;
    if (!line) {
      const geo = new THREE.BufferGeometry();
      const positions = new Float32Array(2 * 3);
      geo.setAttribute('position', new THREE.BufferAttribute(positions, 3));

      const mat = new THREE.LineBasicMaterial({
        color: new THREE.Color(1, 0, 0),
        linewidth: 10,
        depthTest: true,
      });
      line = new THREE.Line(geo, mat);
      scene().add(line);
    }

    const positions = line.geometry.attributes.position;
    positions.array.set(value[0].toArray(), 0);
    positions.array.set(value[1].toArray(), 3);
    positions.needsUpdate = true;

    // Recompute box and sphere to ensure the new position doesn't get culled from the camera
    line.geometry.computeBoundingBox();
    line.geometry.computeBoundingSphere();

    markLineCollisions(mesh, xiMeshBuffer(), preparedData()!, value[0], value[1]);

    return line;
  }));

  function castRayOntoMesh(): RayHit[] | undefined {
    const mesh = zoneMesh();
    if (!mesh) {
      return;
    }

    raycaster.setFromCamera(cameraMouse, camera());
    const intersections = raycaster.intersectObject(mesh, false);
    if (intersections.length == 0) {
      return;
    }

    let result = [];
    for (const int of intersections) {
      const p = int.point;
      const face = int.face ? { a: int.face.a, b: int.face.b, c: int.face.c } : undefined;
      result.push({
        x: p.x,
        y: p.y,
        z: p.z,
        object: int.object,
        index: int.index!,
        faceIndex: int.faceIndex!,
        face,
      });
    }
    return result;
  }

  // Setup labels
  const labelRenderer = new CSS2DRenderer();
  labelRenderer.setSize(innerWidth, innerHeight);
  labelRenderer.domElement.style.position = "absolute";
  labelRenderer.domElement.style.top = "0px";
  labelRenderer.domElement.style.pointerEvents = "none";

  const labelDiv = document.createElement("div");
  labelDiv.style.padding = "0.1rem 0.4rem";
  labelDiv.style.background = "rgba(0,0,0,0.7)";
  labelDiv.style.color = "0xFFFFFF";
  const label = new CSS2DObject(labelDiv);
  label.visible = false;

  createEffect(on(getColorKind, (colorKind) => {
    const mesh = zoneMesh();
    if (mesh) {
      colorMesh(mesh, preparedData()!, colorKind);
    }
  }));

  const colorKindButton = (colorKind: ColorKind, name: string) => (
    <button
      classList={{ "active": getColorKind() == colorKind }}
      onClick={() => {
        setColorKind(colorKind);
      }}>
      {name}
    </button>
  );

  const positionField = (getter: () => THREE.Vector3 | undefined, setter: (val: THREE.Vector3) => THREE.Vector3) => {
    let copyTimer: number | undefined;

    return <>
      <input class="w-24 text-center hide-spin-buttons" type="number" lang="en-US"
        value={getter()?.x}
        placeholder="x"
        onInput={(e) => {
          let pos = getter()?.clone() ?? new THREE.Vector3();
          pos.x = parseFloat(e.target.value);
          setter(pos);
        }}></input>

      <input class="w-24 text-center hide-spin-buttons" type="number" lang="en-US"
        value={getter()?.y}
        placeholder="y"
        onInput={(e) => {
          let pos = getter()?.clone() ?? new THREE.Vector3();
          pos.y = parseFloat(e.target.value);
          setter(pos);
        }}></input>

      <input class="w-24 text-center hide-spin-buttons" type="number" lang="en-US"
        value={getter()?.z}
        placeholder="z"
        onInput={(e) => {
          let pos = getter()?.clone() ?? new THREE.Vector3();
          pos.z = parseFloat(e.target.value);
          setter(pos);
        }}></input>

      <button class="w-20" onClick={(e) => {
        const pos = getter();
        if (!pos) {
          return;
        }
        let value;
        if (e.shiftKey) {
          value = `${pos.x} ${pos.y} ${pos.z}`;
        } else {
          value = `${pos.x},${pos.y},${pos.z}`;
        }
        writeText(value);

        e.target.textContent = "Copied"
        if (copyTimer) {
          clearTimeout(copyTimer);
        }
        copyTimer = setTimeout(() => {
          e.target.textContent = "Copy"
          copyTimer = undefined;
        }, 2000);
      }}>
        Copy
      </button>


      <button class="w-20" onClick={async () => {
        let coords = parseCoordinatesToVector3(await readText());
        if (coords) {
          setter(coords);
        }
      }}>
        Paste
      </button>
    </>
  };

  return (
    <div class="flex flex-col flex-grow h-full">
      <h1>
        Zone Data: {zoneId}

        <Switch>
          <Match when={xiMeshBuffer.loading}>
            <span class="px-3 text-green-200 text-sm">[Loading...]</span>
          </Match>
          <Match when={xiMeshBuffer.error}>
            <span class="px-3 text-red-500">[Error: {xiMeshBuffer.error}]</span>
          </Match>
        </Switch>
      </h1>
      <hr />

      <div class="w-full h-3/4 relative">
        <canvas class="block w-full h-full" ref={canvasElement}></canvas>
        <div
          class="absolute hidden p-1 text-white bg-black pointer-events-none rounded font-mono opacity-70 text-sm noselect"
          ref={coordLabelRef}
        >
        </div>
        <ZoneInfoBox targetInfo={getTargetInfo()}></ZoneInfoBox>
        <div
          class="absolute top-0 pointer-events-none"
          ref={labelRendererElement}
        >
        </div>

        {stats.dom}
      </div>

      <div class="flex items-start justify-around pt-3 flex-wrap">

        <div class="flex flex-col items-center pb-5">
          <span class="font-semibold">Colors</span>
          <div class="flex items-center justify-center gap-2 flex-wrap">
            {colorKindButton(ColorKind.None, "Clear")}
            {colorKindButton(ColorKind.Barriers, "Barriers")}
            {colorKindButton(ColorKind.Materials, "Materials")}
            {colorKindButton(ColorKind.Maps, "Map")}
            {colorKindButton(ColorKind.IsRoofed, "Roofed")}
          </div>
        </div>


        <div class="flex flex-col items-center">
          <span class="font-semibold">Ray testing</span>
          <div class="flex items-center gap-1 flex-wrap">
            <span class="w-32">Start (CTRL+click):</span>
            {positionField(getStartPos, setStartPos)}
          </div>

          <div class="flex items-center gap-1 flex-wrap">
            <span class="w-32">End (SHIFT+click):</span>
            {positionField(getEndPos, setEndPos)}
          </div>
        </div>

      </div>
    </div >
  );
}

export interface RayHit {
  x: number;
  y: number;
  z: number;
  index?: number;
  faceIndex?: number;
  object: THREE.Object3D,
  face?: {
    a: number;
    b: number;
    c: number;
  }
}

export default ZoneData;
