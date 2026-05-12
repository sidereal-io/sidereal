#!/usr/bin/env node

/**
 * Test script to check sidecar directory permissions in Docker
 * Run this inside the container to diagnose permission issues
 */

const fs = require('fs');
const path = require('path');

console.log('=== Sidecar Directory Permission Test ===\n');

// Check environment variables
console.log('Environment Variables:');
console.log(`XMP_SIDECAR_PATH: ${process.env.XMP_SIDECAR_PATH || 'not set'}`);
console.log(`NODE_ENV: ${process.env.NODE_ENV || 'not set'}`);
console.log(`USER: ${process.env.USER || 'not set'}`);
console.log(`UID: ${process.env.UID || 'not set'}`);
console.log(`GID: ${process.env.GID || 'not set'}\n`);

// Determine sidecar directory
const sidecarDir = process.env.XMP_SIDECAR_PATH || '/app/sidecars';
console.log(`Testing sidecar directory: ${sidecarDir}\n`);

// Check if directory exists
console.log('1. Checking if directory exists:');
try {
  const exists = fs.existsSync(sidecarDir);
  console.log(`   Directory exists: ${exists}`);
  
  if (exists) {
    const stats = fs.statSync(sidecarDir);
    console.log(`   Is directory: ${stats.isDirectory()}`);
    console.log(`   Permissions: ${stats.mode.toString(8)}`);
    console.log(`   Owner: ${stats.uid}`);
    console.log(`   Group: ${stats.gid}`);
  }
} catch (error) {
  console.log(`   Error checking directory: ${error.message}`);
}
console.log('');

// Try to create directory if it doesn't exist
console.log('2. Attempting to create directory:');
try {
  if (!fs.existsSync(sidecarDir)) {
    fs.mkdirSync(sidecarDir, { recursive: true });
    console.log(`   Successfully created directory: ${sidecarDir}`);
  } else {
    console.log('   Directory already exists');
  }
} catch (error) {
  console.log(`   Error creating directory: ${error.message}`);
}
console.log('');

// Test write permissions
console.log('3. Testing write permissions:');
try {
  const testFile = path.join(sidecarDir, '.test-write');
  fs.writeFileSync(testFile, 'test content', 'utf8');
  console.log(`   Successfully wrote test file: ${testFile}`);
  
  // Clean up test file
  fs.unlinkSync(testFile);
  console.log('   Successfully deleted test file');
} catch (error) {
  console.log(`   Error testing write permissions: ${error.message}`);
}
console.log('');

// Test actual XMP file creation
console.log('4. Testing XMP file creation:');
try {
  const testXmpFile = path.join(sidecarDir, 'test-image.jpg.xmp');
  const xmpContent = `<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Sidereal">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/">
      <dc:title>Test Image</dc:title>
      <dc:description>Test XMP sidecar file</dc:description>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>`;
  
  fs.writeFileSync(testXmpFile, xmpContent, 'utf8');
  console.log(`   Successfully wrote XMP file: ${testXmpFile}`);
  
  // Clean up test file
  fs.unlinkSync(testXmpFile);
  console.log('   Successfully deleted test XMP file');
} catch (error) {
  console.log(`   Error testing XMP file creation: ${error.message}`);
}
console.log('');

// List directory contents
console.log('5. Directory contents:');
try {
  const files = fs.readdirSync(sidecarDir);
  if (files.length === 0) {
    console.log('   Directory is empty');
  } else {
    console.log('   Files in directory:');
    files.forEach(file => {
      const filePath = path.join(sidecarDir, file);
      const stats = fs.statSync(filePath);
      console.log(`     ${file} (${stats.isDirectory() ? 'dir' : 'file'})`);
    });
  }
} catch (error) {
  console.log(`   Error listing directory contents: ${error.message}`);
}
console.log('');

console.log('=== Test Complete ==='); 