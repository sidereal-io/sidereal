---
name: frontend-css
description: Guide for writing and reviewing CSS in frontend web projects. Use this skill when creating new styles, styling components, refactoring CSS, or reviewing CSS code for adherence to project standards and best practices.
---

# CSS Frontend

## Overview

This skill provides guidance for writing production-quality CSS in frontend web projects. Apply these standards when creating new styles, styling components, or reviewing CSS code to ensure consistency, maintainability, and performance.

## CSS Standards

Follow these core principles when working with CSS:

### 1. Consistent Methodology

Apply and stick to the project's consistent CSS methodology throughout the codebase. Common methodologies include:
- **Utility-first** (Tailwind, UnoCSS) - Compose styles from atomic utility classes
- **BEM** (Block Element Modifier) - Use structured naming conventions
- **CSS Modules** - Leverage scoped styles with local class names
- **CSS-in-JS** - Write styles in JavaScript (styled-components, Emotion)

Identify the methodology in use and maintain consistency. Mixing methodologies creates confusion and maintenance burden.

### 2. Avoid Overriding Framework Styles

Work with the framework's patterns rather than fighting against them with excessive overrides. Framework styles exist for good reasons:
- They handle edge cases and browser inconsistencies
- They maintain accessibility features
- They ensure responsive behavior

When framework styles seem insufficient:
- First, check if there's a built-in way to achieve the desired result
- Use framework extension points (theme customization, variants)
- Only override as a last resort, and document why

### 3. Maintain Design System

Establish and document design tokens for consistency across the application. Design tokens include:
- **Colors** - Primary, secondary, semantic colors (success, warning, error), text colors, backgrounds
- **Spacing** - Consistent scale (4px, 8px, 16px, 24px, 32px, etc.)
- **Typography** - Font families, sizes, weights, line heights
- **Breakpoints** - Consistent responsive design breakpoints
- **Shadows** - Elevation system for depth
- **Border radii** - Consistent corner rounding

Use CSS custom properties (variables) or framework tokens to reference these values. Never hardcode arbitrary values.

**Good:**
```css
color: var(--color-primary);
padding: var(--spacing-4);
border-radius: var(--radius-md);
```

**Avoid:**
```css
color: #3b82f6;
padding: 17px;
border-radius: 6px;
```

### 4. Minimize Custom CSS

Leverage framework utilities and components to reduce custom CSS maintenance burden. Custom CSS:
- Increases bundle size
- Requires ongoing maintenance
- Can create inconsistencies
- May not be responsive or accessible by default

Before writing custom CSS:
1. Check if framework utilities can achieve the result
2. Check if composing existing utilities works
3. Consider if this is a reusable pattern worth adding to the design system
4. Only write custom CSS when necessary

### 5. Performance Considerations

Optimize CSS for production to ensure fast load times:
- **Enable CSS purging/tree-shaking** - Remove unused styles in production builds
- **Minimize specificity wars** - Keep selectors simple and avoid `!important`
- **Avoid expensive selectors** - Universal selectors (*), deep descendant selectors
- **Use modern CSS features** - Flexbox, Grid, custom properties instead of JavaScript solutions
- **Critical CSS** - Inline critical above-the-fold styles for faster first paint

## Writing New CSS

When writing new styles for components or pages:

### Step 1: Identify the Design Requirements

Clearly understand what needs to be styled:
- Layout structure (flexbox, grid, positioning)
- Visual appearance (colors, typography, spacing)
- Interactive states (hover, focus, active, disabled)
- Responsive behavior (mobile, tablet, desktop)
- Accessibility needs (focus indicators, contrast ratios)

### Step 2: Check Existing Patterns

Before writing any CSS:
1. Search the codebase for similar components or patterns
2. Review the design system for existing tokens and utilities
3. Check if the framework provides built-in solutions
4. Identify opportunities to reuse existing styles

### Step 3: Choose the Appropriate Approach

Select the approach based on the project's methodology:

**For utility-first frameworks:**
- Compose styles from atomic utility classes
- Extract repeated patterns into components when needed
- Use framework configuration for custom values

**For BEM/traditional CSS:**
- Follow the naming convention (block__element--modifier)
- Keep specificity low
- Organize styles logically (layout → visual → states)

**For CSS-in-JS:**
- Colocate styles with components
- Use theme tokens for consistency
- Leverage dynamic styling when needed

### Step 4: Implement with Best Practices

Write CSS that is:
- **Readable** - Clear class names, logical organization
- **Maintainable** - No magic numbers, documented complex logic
- **Accessible** - Sufficient contrast, visible focus states, responsive text
- **Responsive** - Mobile-first approach, proper breakpoints
- **Performant** - Minimal specificity, no redundant declarations

### Step 5: Test Across Contexts

Verify the CSS works in different scenarios:
- Multiple screen sizes (mobile, tablet, desktop)
- Different browsers (Chrome, Firefox, Safari)
- Different states (hover, focus, disabled, error)
- With real content (long text, short text, images, no data)
- With accessibility tools (keyboard navigation, screen readers)

## Reviewing CSS

When reviewing CSS code for adherence to standards:

### Step 1: Check Methodology Consistency

Verify the CSS follows the project's established methodology:
- Utility-first projects should use utility classes, not custom CSS
- BEM projects should follow naming conventions
- CSS Modules should use local scoping
- CSS-in-JS should colocate with components

Flag any deviations or mixing of methodologies.

### Step 2: Verify Design System Usage

Ensure design tokens are used instead of arbitrary values:
- Colors come from the color palette
- Spacing uses the spacing scale
- Typography follows the type scale
- Border radii match the system
- Shadows use the elevation system

Flag any hardcoded values that should use tokens.

### Step 3: Assess Framework Integration

Check if the CSS works with the framework rather than against it:
- No unnecessary framework overrides
- Uses framework extension points when available
- Respects framework conventions
- Leverages framework features (utilities, components, variants)

Flag any code that fights the framework.

### Step 4: Evaluate Performance Impact

Review for performance considerations:
- No overly specific selectors
- No excessive nesting
- No redundant declarations
- Styles will be purged/tree-shaken in production
- No expensive CSS properties without justification

Flag any performance concerns.

### Step 5: Check Accessibility and Responsiveness

Verify the CSS is accessible and responsive:
- Sufficient color contrast (WCAG AA minimum)
- Visible focus indicators
- No fixed pixel widths that break responsive layout
- Proper responsive breakpoints
- Text can be resized without breaking layout
- No reliance on color alone to convey information

Flag any accessibility or responsive design issues.

### Step 6: Provide Constructive Feedback

When issues are found:
1. Clearly identify what doesn't align with standards
2. Explain why it matters (maintainability, performance, accessibility)
3. Suggest specific improvements with examples
4. Reference the relevant standard or principle
5. Acknowledge what was done well

## Common Patterns and Solutions

### Center Content

**Flexbox (preferred for single-axis centering):**
```css
display: flex;
justify-content: center; /* horizontal */
align-items: center; /* vertical */
```

**Grid (preferred for full-area centering):**
```css
display: grid;
place-items: center;
```

### Responsive Typography

Use responsive units and clamp for fluid typography:
```css
font-size: clamp(1rem, 2vw + 0.5rem, 1.5rem);
```

### Card Component Layout

Leverage framework utilities or use modern CSS:
```css
.card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  padding: var(--spacing-6);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  box-shadow: var(--shadow-md);
}
```

### Accessible Focus States

Always provide visible focus indicators:
```css
:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: 2px;
}
```

### Dark Mode Support

Use CSS custom properties that can be updated:
```css
:root {
  --color-text: #1a1a1a;
  --color-background: #ffffff;
}

[data-theme="dark"] {
  --color-text: #f5f5f5;
  --color-background: #1a1a1a;
}
```

## Summary

When working with CSS in frontend projects:
1. **Identify and follow the project's CSS methodology** - Consistency is critical
2. **Use the design system** - Tokens over arbitrary values
3. **Leverage the framework** - Work with it, not against it
4. **Minimize custom CSS** - Compose from existing utilities when possible
5. **Optimize for performance** - Enable purging, minimize specificity
6. **Ensure accessibility** - Proper contrast, focus states, responsive design
7. **Review thoroughly** - Test across devices, browsers, and states
