/**
 * 05 — Form Awareness
 *
 * Perceive and understand form structure without any site-specific knowledge.
 * The agent discovers what the form is asking, identifies required fields,
 * understands input types, and reports the form's purpose.
 *
 * This is how an agent should approach any form — through perception,
 * not hardcoded field matchers.
 *
 * Usage: bun run examples/05-form-awareness.ts [url]
 */

import { TivanaClient, type Element } from "tivana";

const url = process.argv[2] || "https://httpbin.org/forms/post";

interface FormField {
  id: string;
  role: string;
  label: string;
  type: string;
  required: boolean;
  value: string;
  visible: boolean;
  interactable: boolean;
  options?: string[];
}

interface FormAnalysis {
  url: string;
  title: string;
  fields: FormField[];
  submitButton: Element | null;
  requiredCount: number;
  filledCount: number;
  purpose: string;
}

function analyzeForm(page: { url: string; title: string | null }, elements: Element[]): FormAnalysis {
  const formRoles = [
    "text", "email", "password", "search", "tel", "number", "url", "date",
    "textarea", "select", "checkbox", "radio", "combobox", "searchbox",
    "textbox", "spinbutton", "slider", "switch",
  ];

  const fields: FormField[] = elements
    .filter((e) => formRoles.includes(e.role))
    .map((e) => ({
      id: e.id,
      role: e.role,
      label: e.name || "(unlabeled)",
      type: e.role,
      required: e.required || false,
      value: e.value || "",
      visible: e.visible,
      interactable: (e as any).interactable,
    }));

  const submitButton = elements.find(
    (e) =>
      e.role === "button" &&
      e.name &&
      (e.name.toLowerCase().includes("submit") ||
        e.name.toLowerCase().includes("send") ||
        e.name.toLowerCase().includes("save") ||
        e.name.toLowerCase().includes("continue") ||
        e.name.toLowerCase().includes("sign") ||
        e.name.toLowerCase().includes("create") ||
        e.name.toLowerCase().includes("register") ||
        e.name.toLowerCase().includes("apply"))
  ) || null;

  const requiredCount = fields.filter((f) => f.required).length;
  const filledCount = fields.filter((f) => f.value.length > 0).length;

  // Infer purpose from field labels
  const labels = fields.map((f) => f.label.toLowerCase()).join(" ");
  let purpose = "Unknown form";
  if (labels.includes("password") && labels.includes("email")) {
    purpose = labels.includes("confirm") || labels.includes("register")
      ? "Registration form"
      : "Login form";
  } else if (labels.includes("search")) {
    purpose = "Search form";
  } else if (labels.includes("address") || labels.includes("city")) {
    purpose = "Address/shipping form";
  } else if (labels.includes("card") || labels.includes("payment")) {
    purpose = "Payment form";
  } else if (labels.includes("name") && labels.includes("email")) {
    purpose = "Contact form";
  } else if (labels.includes("message") || labels.includes("comment")) {
    purpose = "Message/comment form";
  } else if (labels.includes("experience") || labels.includes("resume")) {
    purpose = "Job application form";
  }

  return {
    url: page.url,
    title: page.title || "(no title)",
    fields,
    submitButton,
    requiredCount,
    filledCount,
    purpose,
  };
}

async function main() {
  const client = new TivanaClient({ url: "ws://localhost:9876" });
  await client.connect();
  await client.createSession();

  console.log(`\n📝 Form Awareness: ${url}\n`);
  await client.navigate(url);

  const page = await client.pageState();
  const elements = await client.elements();

  console.log(`📄 ${page.title}`);
  console.log(`🧩 ${elements.length} elements\n`);

  const analysis = analyzeForm(page, elements);

  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  console.log(`  Form Analysis`);
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  console.log(`  Purpose: ${analysis.purpose}`);
  console.log(`  Fields: ${analysis.fields.length}`);
  console.log(`  Required: ${analysis.requiredCount}`);
  console.log(`  Pre-filled: ${analysis.filledCount}`);
  console.log(`  Submit button: ${analysis.submitButton ? `${analysis.submitButton.id} "${analysis.submitButton.name}"` : "not found"}`);
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n`);

  if (analysis.fields.length === 0) {
    console.log(`  No form fields detected on this page.\n`);
  } else {
    console.log(`📋 Fields:\n`);
    for (const field of analysis.fields) {
      const flags = [
        field.required ? "REQUIRED" : "",
        field.value ? `value="${field.value.slice(0, 20)}"` : "",
        !field.visible ? "HIDDEN" : "",
        !field.interactable ? "NOT INTERACTABLE" : "",
      ]
        .filter(Boolean)
        .join(" | ");

      console.log(`  ${field.id} [${field.role}] "${field.label}"`);
      if (flags) console.log(`     ${flags}`);
    }
  }

  // What an agent would need to fill this form
  if (analysis.fields.length > 0) {
    console.log(`\n🤖 Agent requirements to fill this form:\n`);
    for (const field of analysis.fields.filter((f) => f.visible && f.interactable)) {
      const needsValue = field.value.length === 0;
      console.log(`  ${needsValue ? "❓" : "✅"} ${field.label} [${field.role}]${field.required ? " (required)" : ""}`);
    }
  }

  console.log(`\n✅ Analysis complete.\n`);

  await client.closeSession();
  client.disconnect();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
