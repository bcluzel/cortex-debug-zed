#!/usr/bin/env node
/**
 * Generates debug_adapter_schemas/cortex-debug-zed.json from cortex-debug's
 * package.json contributes.debuggers.configurationAttributes.
 *
 * Usage: node generate_schema.js <path-to-cortex-debug/package.json> <output-path>
 */

const fs = require("fs");
const path = require("path");

const [, , packageJsonPath, outputPath] = process.argv;
if (!packageJsonPath || !outputPath) {
  console.error(
    "Usage: generate_schema.js <package.json> <output.json>"
  );
  process.exit(1);
}

const pkg = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
const debuggers = pkg.contributes?.debuggers;
if (!debuggers) {
  console.error("No contributes.debuggers found in package.json");
  process.exit(1);
}

const attrs = debuggers.configurationAttributes ?? debuggers[0]?.configurationAttributes;
if (!attrs) {
  console.error("No configurationAttributes found");
  process.exit(1);
}

const launch = attrs.launch ?? {};
const attach = attrs.attach ?? {};

// Merge launch and attach properties; launch takes precedence.
// Add a discriminating "request" property.
const mergedProperties = {
  request: {
    type: "string",
    description: "Debug request type: 'launch' to start a new process, 'attach' to connect to a running one.",
    enum: ["launch", "attach"],
    default: "launch",
  },
  ...attachProperties(attach.properties ?? {}),
  ...attachProperties(launch.properties ?? {}),
};

// Collect required fields (union of both, minus request which has a default)
const required = [
  ...(launch.required ?? []),
];

function attachProperties(props) {
  // Strip any VSCode-specific $ref / markdownDescription fields that are not
  // valid JSON Schema draft-07 and may confuse consumers.
  const out = {};
  for (const [key, val] of Object.entries(props)) {
    out[key] = cleanProperty(val);
  }
  return out;
}

function cleanProperty(val) {
  if (typeof val !== "object" || val === null) return val;
  const cleaned = {};
  for (const [k, v] of Object.entries(val)) {
    // Drop VSCode-specific keys
    if (k === "markdownDescription" || k === "enumDescriptions" || k === "markdownEnumDescriptions") continue;
    if (k === "properties" && typeof v === "object") {
      cleaned[k] = attachProperties(v);
    } else if (k === "items" && typeof v === "object") {
      cleaned[k] = cleanProperty(v);
    } else if (k === "anyOf" || k === "oneOf" || k === "allOf") {
      cleaned[k] = Array.isArray(v) ? v.map(cleanProperty) : cleanProperty(v);
    } else {
      cleaned[k] = v;
    }
  }
  return cleaned;
}

const schema = {
  $schema: "http://json-schema.org/draft-07/schema#",
  title: "Cortex Debug Session Configuration",
  description:
    "Configuration for the cortex-debug debug adapter (GDB-based embedded ARM/Cortex-M debugger).",
  type: "object",
  properties: mergedProperties,
  required: required.length > 0 ? required : undefined,
};

// Remove undefined fields (e.g. required when empty)
const output = JSON.stringify(schema, (_, v) => (v === undefined ? undefined : v), 2);

fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, output, "utf8");
console.log(`[generate_schema] Written to ${outputPath}`);
