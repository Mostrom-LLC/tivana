/**
 * Tivana SDK Example
 * 
 * Demonstrates connecting to the Tivana runtime, creating a session,
 * navigating to a page, perceiving elements, and taking actions.
 * 
 * Run with: bun run example.ts
 */

import { TivanaClient } from './src/index.js';
import type { Mutation, PageState, Element } from './src/index.js';

const TIVANA_URL = process.env.TIVANA_URL ?? 'ws://localhost:9222';

async function main() {
  console.log('🚀 Tivana SDK Example\n');

  // Create client with custom options
  const client = new TivanaClient({
    timeout: 30000,
    autoReconnect: true,
  });

  try {
    // =========================================================================
    // Step 1: Connect to the runtime
    // =========================================================================
    console.log(`📡 Connecting to Tivana at ${TIVANA_URL}...`);
    await client.connect(TIVANA_URL);
    console.log('✅ Connected!\n');

    // =========================================================================
    // Step 2: Create a browser session
    // =========================================================================
    console.log('🌐 Creating browser session...');
    const sessionId = await client.createSession();
    console.log(`✅ Session created: ${sessionId}\n`);

    // =========================================================================
    // Step 3: Register mutation listener
    // =========================================================================
    client.onMutation((mutations: Mutation[], timestamp: number) => {
      console.log(`\n🔄 Page mutation at ${new Date(timestamp).toISOString()}`);
      for (const mutation of mutations) {
        switch (mutation.type) {
          case 'added':
            console.log(`  + Added: ${mutation.element.role} "${mutation.element.label}"`);
            break;
          case 'removed':
            console.log(`  - Removed: ${mutation.elementId}`);
            break;
          case 'changed':
            console.log(`  ~ Changed: ${mutation.elementId}`, mutation.changes);
            break;
        }
      }
    });

    // =========================================================================
    // Step 4: Navigate to a page
    // =========================================================================
    console.log('🔗 Navigating to https://example.com...');
    const state = await client.navigate('https://example.com');
    printPageState(state);

    // =========================================================================
    // Step 5: Perceive - Get all elements
    // =========================================================================
    console.log('👁️ Perceiving page elements...');
    const elements = await client.elements();
    printElements(elements);

    // =========================================================================
    // Step 6: Act - Find and click a link
    // =========================================================================
    const link = elements.find(el => el.role === 'link');
    if (link) {
      console.log(`\n🖱️ Clicking link: "${link.label}" (${link.id})`);
      const stateAfterClick = await client.click(link.id);
      console.log(`✅ Clicked! New page: ${stateAfterClick.title}\n`);
    }

    // =========================================================================
    // Step 7: Type into a text field (if exists)
    // =========================================================================
    const textbox = elements.find(el => el.role === 'textbox');
    if (textbox) {
      console.log(`⌨️ Typing into textbox: "${textbox.label}" (${textbox.id})`);
      await client.type('Hello from Tivana!', textbox.id);
      console.log('✅ Typed!\n');
    }

    // =========================================================================
    // Step 8: Scroll an element into view
    // =========================================================================
    const lastElement = elements[elements.length - 1];
    if (lastElement) {
      console.log(`📜 Scrolling to element: ${lastElement.id}`);
      await client.scroll(lastElement.id);
      console.log('✅ Scrolled!\n');
    }

    // =========================================================================
    // Step 9: Get final page state
    // =========================================================================
    console.log('📊 Final page state:');
    const finalState = await client.pageState();
    printPageState(finalState);

    // =========================================================================
    // Step 10: Clean up
    // =========================================================================
    console.log('🧹 Closing session...');
    await client.closeSession();
    console.log('✅ Session closed!\n');

  } catch (error) {
    console.error('❌ Error:', error);
  } finally {
    // Always disconnect
    console.log('👋 Disconnecting...');
    client.disconnect();
    console.log('✅ Done!');
  }
}

function printPageState(state: PageState): void {
  console.log(`\n📄 Page State:`);
  console.log(`   URL: ${state.url}`);
  console.log(`   Title: ${state.title}`);
  console.log(`   Elements: ${state.elements.length}`);
  console.log(`   Viewport: ${state.viewport.width}x${state.viewport.height}`);
  console.log(`   Scroll: (${state.scrollPosition.x}, ${state.scrollPosition.y})`);
  console.log(`   Focused: ${state.focusedElement ?? 'none'}`);
  console.log();
}

function printElements(elements: Element[]): void {
  console.log(`\n🧩 Elements (${elements.length} total):`);
  
  // Show first 10 elements
  const displayCount = Math.min(elements.length, 10);
  for (let i = 0; i < displayCount; i++) {
    const el = elements[i];
    const label = el.label ? `"${el.label}"` : '';
    const state = [
      el.focused ? '🎯' : '',
      el.interactable ? '👆' : '',
      !el.visible ? '👻' : '',
    ].filter(Boolean).join('');
    
    console.log(`   ${el.id}: ${el.role} ${label} ${state}`);
    
    if (el.bounds) {
      console.log(`      📐 ${el.bounds.width}x${el.bounds.height} at (${el.bounds.x}, ${el.bounds.y})`);
    }
    
    if (el.font) {
      console.log(`      🔤 ${el.font.family} ${el.font.size} ${el.font.color}`);
    }
  }
  
  if (elements.length > displayCount) {
    console.log(`   ... and ${elements.length - displayCount} more`);
  }
  console.log();
}

// Run the example
main().catch(console.error);
