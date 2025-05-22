import * as THREE from "three";

export function cleanupNode(node: THREE.Object3D) {
  if (node instanceof THREE.Mesh) {
    node.geometry.dispose();

    let materials = node.material instanceof Array ? node.material : [node.material];
    for (const material of materials) {
      material.dispose();
    }
  }
  for (const child of node.children) {
    cleanupNode(child);
  }
  node.clear();
}

export function roundDecimals(num: number, decimals: number): number {
  return parseFloat(num.toFixed(decimals))
}

/**
 * Parses a string of coordinates into a THREE.Vector3.
 * Supports various formats including comma-separated, space-separated,
 * and object-like (e.g., "x: 1, y: 2, z: 3").
 * Handles both comma and dot as decimal separators.
 *
 * @param {string} coordString The string containing the coordinates.
 * @returns {THREE.Vector3 | null} A THREE.Vector3 object if parsing is successful, otherwise null.
 */
export function parseCoordinatesToVector3(coordString: string): THREE.Vector3 | null {
  if (!coordString || typeof coordString !== 'string') {
    // Return null if the input is not a valid string.
    return null;
  }

  // Normalize decimal separators: replace commas with dots, but only if they are
  // likely to be decimal separators (i.e., followed by a digit).
  // This prevents replacing commas used as separators between coordinates in the "x: y: z:" format.
  let normalizedString = coordString.replace(/(\d),(\d)/g, '$1.$2');

  // Attempt to parse as key-value pairs (e.g., "x: -3.221, y: 15.030, z: -232.111")
  // The regex captures the numeric values after 'x:', 'y:', and 'z:'.
  const keyValueMatch = normalizedString.match(/(?:x\s*:\s*(-?\d+\.?\d*))\s*(?:,\s*y\s*:\s*(-?\d+\.?\d*))\s*(?:,\s*z\s*:\s*(-?\d+\.?\d*))/i);
  if (keyValueMatch) {
    // Parse the captured strings into floating-point numbers.
    const x = parseFloat(keyValueMatch[1]);
    const y = parseFloat(keyValueMatch[2]);
    const z = parseFloat(keyValueMatch[3]);
    // Check if all parsed values are valid numbers.
    if (!isNaN(x) && !isNaN(y) && !isNaN(z)) {
      return new THREE.Vector3(x, y, z);
    }
  }

  // Attempt to parse as space or comma-separated numbers (e.g., "-3.221, 15.030, -232.111" or "-3.221 15.030 -232.111")
  // This regex finds all sequences of numbers, optionally with a leading minus sign and a decimal part.
  const numberMatches = normalizedString.match(/-?\d+\.?\d*/g);
  if (numberMatches && numberMatches.length >= 3) {
    // Extract the first three numbers found.
    const x = parseFloat(numberMatches[0]);
    const y = parseFloat(numberMatches[1]);
    const z = parseFloat(numberMatches[2]);
    // Check if all extracted values are valid numbers.
    if (!isNaN(x) && !isNaN(y) && !isNaN(z)) {
      return new THREE.Vector3(x, y, z);
    }
  }

  // If no format matches, return null indicating failure to parse.
  return null;
}