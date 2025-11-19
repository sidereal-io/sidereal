---
name: frontend-accessibility
description: Guide for building and reviewing accessible frontend interfaces. Use this skill when creating new components, pages, or applications that need to meet accessibility standards, or when reviewing code for WCAG 2.1 AA compliance. Covers semantic HTML, keyboard navigation, screen readers, ARIA, color contrast, and focus management.
---

# Frontend Accessibility

## Overview

This skill provides guidance for building production-quality accessible frontend interfaces that meet WCAG 2.1 AA standards. Apply these principles when creating new components, pages, or applications, and when reviewing existing code to ensure all users can access and interact with web content effectively.

## Core Accessibility Principles

Follow these fundamental principles when building accessible frontends:

### 1. WCAG 2.1 AA Compliance

Strive to meet Web Content Accessibility Guidelines (WCAG) 2.1 Level AA standards. This includes:
- **Perceivable** - Information and UI components must be presentable to users in ways they can perceive
- **Operable** - UI components and navigation must be operable by all users
- **Understandable** - Information and UI operation must be understandable
- **Robust** - Content must be robust enough to be interpreted by assistive technologies

WCAG 2.1 AA is the legally required standard in many jurisdictions and represents the baseline for accessible web experiences.

### 2. Semantic HTML

Use appropriate HTML elements that convey meaning to assistive technologies. Semantic HTML provides:
- Clear document structure for screen readers
- Better keyboard navigation
- Improved SEO
- More maintainable code

**Common semantic elements:**
- `<header>`, `<nav>`, `<main>`, `<article>`, `<section>`, `<aside>`, `<footer>` - Document structure
- `<button>` - Interactive buttons (not `<div>` with click handlers)
- `<a>` - Links and navigation
- `<h1>`-`<h6>` - Heading hierarchy
- `<ul>`, `<ol>`, `<li>` - Lists
- `<form>`, `<label>`, `<input>` - Forms

### 3. Keyboard Navigation

Ensure all interactive elements are accessible via keyboard with visible focus indicators. Users who cannot use a mouse rely entirely on keyboard navigation.

**Requirements:**
- All interactive elements must be focusable (buttons, links, form inputs)
- Logical tab order (use tabindex="0" for custom interactive elements, avoid positive tabindex values)
- Skip links for bypassing repetitive navigation
- No keyboard traps (users can always move focus away)
- Visible focus indicators (never use `outline: none` without replacement)

**Focus styling:**
- Use `:focus-visible` pseudo-class for keyboard-only focus indicators
- Provide sufficient contrast for focus outlines (minimum 3:1)
- Consider box-shadow or outline-offset for better visibility

**Skip links:**
- Provide skip links to bypass repetitive navigation
- Position them visually off-screen but accessible to keyboard users
- Show them on focus

### 4. Color Contrast

Maintain sufficient contrast ratios and never rely solely on color to convey information.

**Contrast requirements (WCAG 2.1 AA):**
- Normal text (< 18pt): 4.5:1 minimum
- Large text (≥ 18pt or ≥ 14pt bold): 3:1 minimum
- UI components and graphics: 3:1 minimum

**Testing:**
- Use browser DevTools contrast checker
- Use online tools like WebAIM Contrast Checker
- Test with color blindness simulators

**Best practices:**
- Use icon + color + text to indicate status (not color alone)
- Never rely solely on color to convey error states, warnings, or success
- Provide text labels or patterns in addition to colors

### 5. Alternative Text

Provide descriptive alt text for images and meaningful labels for all form inputs.

**Image alt text guidelines:**
- Describe the content and function of the image
- Keep it concise (typically < 150 characters)
- For decorative images, use `alt=""` (empty string)
- For complex images (charts, diagrams), provide longer descriptions using `<figcaption>` or `aria-describedby`
- For functional images (buttons), describe the action

**Form labels:**
- Associate labels with inputs using `for` and `id` attributes
- Provide helper text using `aria-describedby`
- Mark required fields clearly (not just with asterisks)
- Use `<fieldset>` and `<legend>` for related form groups

### 6. Screen Reader Testing

Test and verify that all views are accessible on screen reading devices. Screen readers are the primary way many users with visual impairments navigate the web.

**Testing approach:**
- Test with actual screen readers (NVDA on Windows, VoiceOver on Mac/iOS, TalkBack on Android)
- Navigate using only the screen reader (turn off your display if possible)
- Verify all content is announced correctly
- Check that interactive elements are discoverable and actionable
- Ensure dynamic content updates are announced (use ARIA live regions)

**Common screen readers:**
- **NVDA** (Windows) - Free, widely used
- **JAWS** (Windows) - Commercial, very popular
- **VoiceOver** (Mac/iOS) - Built-in to Apple devices
- **TalkBack** (Android) - Built-in to Android devices
- **Narrator** (Windows) - Built-in to Windows

**Basic screen reader testing checklist:**
- [ ] All text content is announced
- [ ] Images have appropriate alt text
- [ ] Form inputs have associated labels
- [ ] Buttons and links are clearly identified
- [ ] Heading structure makes sense
- [ ] Lists are properly marked up
- [ ] Dynamic content updates are announced
- [ ] Modal dialogs trap focus appropriately
- [ ] Error messages are announced

### 7. ARIA When Needed

Use ARIA (Accessible Rich Internet Applications) attributes to enhance complex components when semantic HTML is not sufficient. ARIA fills gaps in HTML semantics but should be used carefully.

**ARIA principles (from the ARIA specification):**
1. **No ARIA is better than bad ARIA** - Incorrect ARIA can make things worse
2. **Prefer semantic HTML** - Use native HTML elements first
3. **Do not change native semantics** - Don't override built-in element roles
4. **All interactive ARIA controls must be keyboard accessible**
5. **Do not use role="presentation" or aria-hidden on focusable elements**

**Common ARIA attributes:**
- `aria-label` - Provides accessible name when no visible label exists
- `aria-labelledby` - References visible text as the accessible name
- `aria-describedby` - Provides additional context or instructions
- `aria-live` - Announces dynamic content updates (polite, assertive, off)
- `aria-expanded` - Indicates if a collapsible element is expanded
- `aria-selected` - Indicates selection state in tabs or lists
- `aria-invalid` - Marks form inputs with validation errors
- `aria-hidden` - Hides decorative content from screen readers

**Common widget roles:**
- `role="dialog"` with `aria-modal="true"` for modal dialogs
- `role="alert"` for important messages
- `role="status"` for status updates
- `role="tablist"`, `role="tab"`, `role="tabpanel"` for tabs
- `role="menu"`, `role="menuitem"` for menus

**Visually hidden content:**
- Use a visually-hidden utility class to hide content visually while keeping it accessible to screen readers
- Never use `display: none` or `visibility: hidden` for content that should be announced

### 8. Logical Heading Structure

Use heading levels (h1-h6) in proper order to create a clear document outline. Headings are the primary way screen reader users navigate content.

**Heading hierarchy rules:**
- Use only one `<h1>` per page (typically the page title)
- Do not skip heading levels (h1 → h2 → h3, not h1 → h3)
- Headings should represent content hierarchy, not visual styling
- Use CSS to style headings, not different heading levels

**Testing heading structure:**
- Use browser extensions like HeadingsMap
- Use screen reader heading navigation (H key in most screen readers)
- Verify the outline makes logical sense

### 9. Focus Management

Manage focus appropriately in dynamic content, modals, and single-page applications. Users should never lose track of where they are on the page.

**Key scenarios requiring focus management:**

**Modal dialogs:**
- Store the element that triggered the modal
- Move focus to the modal when it opens (first focusable element or close button)
- Trap focus within the modal (prevent tabbing outside)
- Return focus to the trigger element when modal closes

**Dynamic content updates:**
- When deleting items, move focus to the next item or a relevant control
- When items are removed, don't let focus disappear
- Provide feedback about what happened

**Single-page application navigation:**
- Move focus to main heading of new view after navigation
- Make the heading focusable with `tabindex="-1"` if needed
- Announce navigation to screen readers using ARIA live regions

**Focus trap pattern:**
- Identify all focusable elements within the container
- On Tab key, cycle focus within the container
- Handle Shift+Tab to cycle backwards

## Implementing Accessible Frontends

When building new components, pages, or features, follow this workflow:

### Step 1: Plan with Accessibility in Mind

Before writing code, consider accessibility requirements:
- What information needs to be conveyed?
- How will keyboard-only users interact with this?
- What should screen readers announce?
- Are there ARIA patterns that apply?
- What are the different states (loading, error, success)?

### Step 2: Use Semantic HTML First

Build the structure using semantic HTML elements:
- Choose the right element for the job (button vs. link, section vs. div)
- Establish proper heading hierarchy
- Use native form elements when possible
- Add landmarks (nav, main, aside, footer)

### Step 3: Add Labels and Text Alternatives

Ensure all content has text alternatives:
- Associate labels with form inputs
- Add alt text to images
- Provide text for icon-only buttons
- Include visually hidden text when needed

### Step 4: Implement Keyboard Navigation

Make everything keyboard accessible:
- Ensure all interactive elements are focusable
- Add visible focus indicators
- Manage focus for dynamic content
- Test tab order
- Implement keyboard shortcuts if needed (with documentation)

### Step 5: Apply ARIA Attributes

Enhance with ARIA where semantic HTML falls short:
- Add roles for custom widgets
- Use aria-label/aria-labelledby for names
- Implement aria-live for dynamic updates
- Set aria-expanded, aria-selected, etc. for stateful widgets
- Add aria-describedby for additional context

### Step 6: Test Thoroughly

Verify accessibility through multiple methods:
- **Keyboard testing** - Navigate using only Tab, Enter, Space, and arrow keys
- **Screen reader testing** - Test with NVDA, JAWS, VoiceOver, or TalkBack
- **Automated testing** - Run axe DevTools, Lighthouse, or pa11y
- **Color contrast** - Check with contrast analyzers
- **Zoom testing** - Test at 200% zoom
- **Real users** - If possible, test with users who rely on assistive technologies

## Reviewing for Accessibility

When reviewing frontend code for accessibility compliance, follow this systematic approach:

### Step 1: Check Semantic HTML

Verify proper use of HTML elements:
- [ ] Interactive elements use `<button>` or `<a>`, not `<div>` with click handlers
- [ ] Document structure uses landmarks (nav, main, aside, footer)
- [ ] Heading hierarchy is logical and sequential
- [ ] Lists use `<ul>`, `<ol>`, `<li>` markup
- [ ] Forms use proper `<form>`, `<label>`, `<input>` elements

Flag any divs or spans used for interactive purposes.

### Step 2: Verify Keyboard Accessibility

Test keyboard navigation:
- [ ] All interactive elements are focusable
- [ ] Tab order is logical
- [ ] Focus indicators are visible
- [ ] No keyboard traps
- [ ] Skip links are provided for navigation
- [ ] Custom widgets support appropriate keyboard patterns

Flag any elements with `tabindex` values greater than 0, or focusable elements without visible focus indicators.

### Step 3: Evaluate ARIA Implementation

Review ARIA usage:
- [ ] ARIA is only used when semantic HTML is insufficient
- [ ] Roles are appropriate for the widget
- [ ] All interactive ARIA elements are keyboard accessible
- [ ] aria-label or aria-labelledby provides names for elements without visible labels
- [ ] aria-live regions announce dynamic content
- [ ] State attributes (aria-expanded, aria-selected) are updated appropriately

Flag any incorrect or unnecessary ARIA, especially role changes on semantic elements.

### Step 4: Check Text Alternatives

Verify all non-text content has alternatives:
- [ ] Images have appropriate alt text
- [ ] Decorative images use `alt=""`
- [ ] Icon buttons have accessible names (aria-label or visually hidden text)
- [ ] Form inputs have associated labels
- [ ] Complex images have extended descriptions

Flag any images without alt attributes or form inputs without labels.

### Step 5: Test Color and Contrast

Verify visual accessibility:
- [ ] Text meets contrast requirements (4.5:1 for normal, 3:1 for large)
- [ ] UI components have 3:1 contrast
- [ ] Information is not conveyed by color alone
- [ ] Color blindness simulators show no loss of information

Flag any contrast failures or color-only indicators.

### Step 6: Verify Focus Management

Check focus handling in dynamic scenarios:
- [ ] Modal dialogs trap focus appropriately
- [ ] Focus returns to trigger element when modals close
- [ ] Deleted items move focus to next item
- [ ] SPA navigation moves focus to new content
- [ ] No focus loss during interactions

Flag any scenarios where focus is lost or trapped incorrectly.

### Step 7: Test with Assistive Technologies

Verify screen reader experience:
- [ ] All content is announced
- [ ] Interactive elements are identified correctly
- [ ] Dynamic updates are announced
- [ ] Navigation landmarks work properly
- [ ] Forms are usable and errors are clear

Flag any content that is not announced or is confusing to navigate.

### Step 8: Run Automated Tools

Use automated accessibility testing:
- **axe DevTools** - Browser extension for comprehensive testing
- **Lighthouse** - Built into Chrome DevTools
- **pa11y** - Command-line testing tool
- **WAVE** - Web accessibility evaluation tool

Flag all issues reported by automated tools, but remember they only catch ~30-40% of accessibility issues.

### Step 9: Provide Constructive Feedback

When accessibility issues are found:
1. Clearly identify what doesn't meet standards
2. Explain the impact on users (who is affected and how)
3. Suggest specific improvements with code examples
4. Reference WCAG success criteria when applicable
5. Acknowledge what was done well

## Summary

When building accessible frontends:
1. **Strive for WCAG 2.1 AA compliance** - It's the legal and ethical standard
2. **Use semantic HTML** - It provides built-in accessibility features
3. **Ensure keyboard navigation** - All functionality must work without a mouse
4. **Maintain color contrast** - Text and UI components must be visible to all
5. **Provide text alternatives** - Images and icons need descriptive alt text
6. **Test with screen readers** - Experience your interface as users with visual impairments do
7. **Apply ARIA carefully** - Enhance complex widgets, but prefer semantic HTML
8. **Structure with headings** - Create a logical, navigable document outline
9. **Manage focus** - Users should always know where they are
10. **Test thoroughly** - Combine automated tools, manual testing, and real user feedback

Accessibility is not optional—it's a fundamental requirement for building inclusive web experiences that work for everyone.
