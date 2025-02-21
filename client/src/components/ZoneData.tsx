import {
  Match,
  Switch,
  createEffect,
  createResource,
  onCleanup,
  onMount,
} from "solid-js";
import * as THREE from "three";
import { useParams } from "@solidjs/router";
import { getZoneModel } from "../custom_bindings";
import CameraControls from "camera-controls";
import * as holdEvent from "hold-event";
import {
  CSS2DObject,
  CSS2DRenderer,
} from "three/examples/jsm/renderers/CSS2DRenderer.js";
import { FlyControls, MapControls } from "three/examples/jsm/Addons.js";
import Stats from 'three/addons/libs/stats.module.js';
import { unwrap } from "../util";

interface ZoneDataProps {}

function ZoneData({}: ZoneDataProps) {
  const params = useParams();
  const zoneId = parseInt(params.id);

  const [modelResource] = createResource(
    async () => {
      const result = unwrap(await getZoneModel({ type: "ZoneData", index: zoneId }));
      return result;
    },
    { initialValue: undefined }
  );

  const clock = new THREE.Clock();

  const stats = new Stats();
  const renderer = new THREE.WebGLRenderer();
  // renderer.setPixelRatio(window.devicePixelRatio);
  renderer.domElement.className = "w-full h-full";
  renderer.setAnimationLoop(animate);

  const raycaster = new THREE.Raycaster();
  const mouse = new THREE.Vector2(1, 1);

  const scene = new THREE.Scene();
  scene.scale.set(1, 1, -1); // FFXI mesh is flipped on the z-axis
  scene.background = new THREE.Color(0x3333333);

  // Grid
  const grid = new THREE.GridHelper(2000, 200, 0xAAAAAA, 0xAAAAAA);
  grid.material.opacity = 0.2;
  grid.material.transparent = true;
  scene.add(grid);

  // Lights
  const hemiLight = new THREE.HemisphereLight(0xE5F5FF, 0xB97A20, 1.5);
  hemiLight.position.set(300, 1000, 300);
  scene.add(hemiLight);

  // Camera
  const camera = new THREE.PerspectiveCamera(60, 1, 0.1, 5000);
  camera.position.set(200, 200, 200);
  camera.lookAt(0, 0, 0);

  // Zone model mesh
  let currentModelMesh: THREE.Mesh[] = [];
  createEffect(() => {
    cleanUpMeshes();

    const buffer = modelResource();
    if (!buffer) {
      return;
    }

    console.time("setup-mesh-" + zoneId);

    const color = new THREE.Color();
    color.setRGB(0.2, 0.2, 0.2);

    const header = new Uint32Array(buffer, 0, 2);
    const triangleCount = header[0];
    const vertexCount = header[1];
    const vertices = new Float32Array(buffer, 8, vertexCount * 3);
    const indices = new Uint32Array(buffer, 8 + vertexCount * 3 * 4, triangleCount * 3);

    const colors = new Uint8Array(vertexCount * 3);
    for (let i = 0; i < vertexCount * 3; i += 3) {
      colors.set([color.r * 255, color.g * 255, color.b * 255], i);
    }

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute("position", new THREE.BufferAttribute(vertices, 3));
    geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3));
    geometry.index = new THREE.Uint32BufferAttribute(indices, 1);

    geometry.computeVertexNormals();
    geometry.computeBoundingBox();

    const material = new THREE.MeshLambertMaterial({
      color: 0x333333,
      vertexColors: true,
      polygonOffset: true,
      polygonOffsetFactor: 1, // positive value pushes polygon further away
      polygonOffsetUnits: 1,
    });

    const mesh = new THREE.Mesh(geometry, material);

    // Add wireframe
    var geo = new THREE.WireframeGeometry(mesh.geometry); // EdgesGeometry or WireframeGeometry
    var mat = new THREE.LineBasicMaterial({
      color: 0x333333,
      transparent: true,
      opacity: 0.2,
      depthTest: true,
    });
    var wireframe = new THREE.LineSegments(geo, mat);
    mesh.add(wireframe);
    
    scene.add(mesh);
    console.timeEnd("setup-mesh");
  });

  function fitCameraToContents() {
    const box = new THREE.Box3();

    // Loop through all children in the scene
    scene.traverse((object) => {
      if (object) {
        box.expandByObject(object); // Include mesh bounding boxes
      }
    });

    const size = new THREE.Vector3();
    const center = new THREE.Vector3();
    box.getSize(size);
    box.getCenter(center);

    // Set camera position to center of bounding box
    camera.position.copy(center);

    // Adjust distance based on size and desired field of view (FOV)
    const distance =
      size.length() / (2 * Math.tan((Math.PI * camera.fov) / 360));
    camera.position.z = distance;

    // Update camera lookAt target (optional)
    camera.lookAt(center);
    console.log(center, distance);
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
  scene.add(label);

  let controls: CameraControls | FlyControls | MapControls = undefined!;

  CameraControls.install({ THREE: THREE });
  onMount(() => {
    addMapControls();
  });

  onCleanup(() => {
    cleanUpMeshes();
    if (controls) {
      controls.dispose();
    }
  });

  function cleanUpMeshes() {
    for (const mesh of currentModelMesh) {
      // Remove old meshes
      scene.remove(mesh);
      console.log("Disposing");
      mesh.geometry.dispose();
      let materials =
        "dispose" in mesh.material ? [mesh.material] : mesh.material;
      for (const material of materials) {
        material.dispose();
      }
    }
  }

  function addMapControls() {
    const mapControls = new MapControls(camera, renderer.domElement);
    controls = mapControls;
  }

  function addCustomCameraControls() {
    controls = new CameraControls(camera, renderer.domElement);
    // Flips left and right mouse buttons
    // controls.mouseButtons = {
    //   left: CameraControls.ACTION.TRUCK,
    //   right: CameraControls.ACTION.ROTATE,
    //   middle: CameraControls.ACTION.DOLLY,
    //   wheel: CameraControls.ACTION.DOLLY,
    // }

    const baseSpeed = 0.01;
    const shiftSpeed = baseSpeed * 3;
    let currentSpeed = baseSpeed;

    window.addEventListener("keydown", (ev) => {
      if (ev.code == "ShiftLeft" || ev.code == "ShiftRight") {
        currentSpeed = shiftSpeed;
      }
    });
    window.addEventListener("keyup", (ev) => {
      if (ev.code == "ShiftLeft" || ev.code == "ShiftRight") {
        currentSpeed = baseSpeed;
      }
    });

    const wKey = new holdEvent.KeyboardKeyHold("KeyW", 16.666);
    const aKey = new holdEvent.KeyboardKeyHold("KeyA", 16.666);
    const sKey = new holdEvent.KeyboardKeyHold("KeyS", 16.666);
    const dKey = new holdEvent.KeyboardKeyHold("KeyD", 16.666);
    aKey.addEventListener("holding", function (event) {
      controls.truck(-1 * currentSpeed * event!.deltaTime, 0, false);
    });
    dKey.addEventListener("holding", function (event) {
      controls.truck(currentSpeed * event!.deltaTime, 0, false);
    });
    wKey.addEventListener("holding", function (event) {
      controls.forward(currentSpeed * event!.deltaTime, false);
    });
    sKey.addEventListener("holding", function (event) {
      controls.forward(-1 * currentSpeed * event!.deltaTime, false);
    });

    const leftKey = new holdEvent.KeyboardKeyHold("ArrowLeft", 100);
    const rightKey = new holdEvent.KeyboardKeyHold("ArrowRight", 100);
    const upKey = new holdEvent.KeyboardKeyHold("ArrowUp", 100);
    const downKey = new holdEvent.KeyboardKeyHold("ArrowDown", 100);
    leftKey.addEventListener("holding", function (event) {
      controls.rotate(
        -0.1 * THREE.MathUtils.DEG2RAD * event!.deltaTime,
        0,
        true
      );
    });
    rightKey.addEventListener("holding", function (event) {
      controls.rotate(
        0.1 * THREE.MathUtils.DEG2RAD * event!.deltaTime,
        0,
        true
      );
    });
    upKey.addEventListener("holding", function (event) {
      controls.rotate(
        0,
        -0.05 * THREE.MathUtils.DEG2RAD * event!.deltaTime,
        true
      );
    });
    downKey.addEventListener("holding", function (event) {
      controls.rotate(
        0,
        0.05 * THREE.MathUtils.DEG2RAD * event!.deltaTime,
        true
      );
    });
  }

  function animate() {
    stats.update();

    const delta = clock.getDelta();
    controls?.update(delta);
    // flyControls?.update(delta);

    if (resizeRendererToDisplaySize()) {
      const canvas = renderer.domElement;
      camera.aspect = canvas.clientWidth / canvas.clientHeight;
      camera.updateProjectionMatrix();
    }

    raycaster.setFromCamera(mouse, camera);

    // const [hovered] = raycaster.intersectObject(mesh);

    // if (hovered) {
    //   const instanceId = hovered.instanceId;

    //   mesh.getColorAt(instanceId!, color);

    //   if (color.equals(white)) {
    //     mesh.setColorAt(instanceId!, color.setHex(Math.random() * 0xffffff));

    //     mesh.instanceColor!.needsUpdate = true;
    //   }

    //   // Setup label
    //   renderer.domElement.className = "hovered";
    //   label.visible = true;
    //   labelDiv.textContent = color.getHexString();

    //   // Move label over hovered element
    //   label.position.set(hovered.point.x, hovered.point.y, hovered.point.z);
    // } else {
    //   // Reset label
    //   renderer.domElement.className = "";
    //   label.visible = false;
    //   labelDiv.textContent = "";
    // }

    renderer.clear();
    renderer.render(scene, camera);
    labelRenderer.render(scene, camera);
  }

  function resizeRendererToDisplaySize() {
    const canvas = renderer.domElement;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    const needResize = canvas.width !== width || canvas.height !== height;
    if (needResize) {
      console.log("Resizing");
      renderer.setSize(width, height, false);
      labelRenderer.setSize(width, height);
      labelRenderer.domElement.style.top = `calc(${renderer.domElement.offsetTop}px - 1.5rem)`;
      stats.dom.style.top = `calc(${renderer.domElement.offsetTop}px)`;
      stats.dom.style.left = `calc(${renderer.domElement.offsetLeft}px)`;
    }
    return needResize;
  }

  renderer.domElement.addEventListener("mousemove", (event) => {
    const canvas = renderer.domElement;
    mouse.x = (2 * event.offsetX) / canvas.offsetWidth - 1;
    mouse.y = (-2 * event.offsetY) / canvas.offsetHeight + 1;
  });

  return (
    <div class="flex flex-col flex-grow">
      <h1>Zone Data: {zoneId}</h1>
      <hr />

      <div class="w-full h-full">
        {renderer.domElement}
        {labelRenderer.domElement}
        {stats.dom}
      </div>

      <Switch>
        <Match when={modelResource.loading}>
          <div>Loading...</div>
        </Match>
        <Match when={modelResource.error}>
          <div>Error: {modelResource.error}</div>
        </Match>
      </Switch>
    </div>
  );
}

export default ZoneData;
