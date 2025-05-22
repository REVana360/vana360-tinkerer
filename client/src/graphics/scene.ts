import * as THREE from "three";

export function setupBaseScene(): THREE.Scene {
  // Background
  const scene = new THREE.Scene();
  scene.scale.set(1, -1, -1); // FFXI mesh is flipped on the y- and z-axis
  scene.background = new THREE.Color(0x3333333);

  // Gridlines
  const grid = new THREE.GridHelper(2000, 200, 0xAAAAAA, 0xAAAAAA);
  grid.material.opacity = 0.2;
  grid.material.transparent = true;
  scene.add(grid);

  // LIGHTS
  const hemiLight = new THREE.HemisphereLight(0xFFFFFF, 0xFFFFFF, 1);
  hemiLight.position.set(300, 1000, 300);
  scene.add(hemiLight);

  return scene;
}
