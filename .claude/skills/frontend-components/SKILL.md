---
name: frontend-components
description: Guide for creating production-quality React UI components using shadcn/ui and Tailwind CSS. Use this skill when building new components, refactoring existing ones, or reviewing component code for adherence to design system standards, accessibility, and best practices.
---

# UI Components

## Overview

This skill provides guidance for creating production-quality React UI components that integrate with shadcn/ui and Tailwind CSS. Apply these principles when building new components, refactoring existing ones, or reviewing component code to ensure consistency, maintainability, accessibility, and alignment with design system standards.

## Core Component Principles

Follow these fundamental principles when building React UI components:

### 1. Component Composition Over Configuration

Build components using composition patterns rather than complex prop-based configuration. Composition creates more flexible, maintainable components.

**Prefer composition:**
```tsx
<Card>
  <CardHeader>
    <CardTitle>Settings</CardTitle>
    <CardDescription>Manage your account settings</CardDescription>
  </CardHeader>
  <CardContent>
    {/* Content here */}
  </CardContent>
  <CardFooter>
    <Button>Save Changes</Button>
  </CardFooter>
</Card>
```

**Avoid complex configuration:**
```tsx
<Card
  title="Settings"
  description="Manage your account settings"
  footer={<Button>Save Changes</Button>}
  headerActions={...}
  showDivider={true}
  variant="elevated"
>
  {/* Content here */}
</Card>
```

**Benefits of composition:**
- More flexible and adaptable to changing requirements
- Easier to understand component structure
- Better TypeScript support and autocomplete
- Follows React and shadcn/ui patterns
- Reduces prop drilling and API complexity

### 2. shadcn/ui Integration

Leverage shadcn/ui components as building blocks rather than creating custom components from scratch. shadcn/ui provides accessible, well-tested primitives that can be customized.

**Component hierarchy:**
1. **Use shadcn/ui primitives first** - Button, Card, Dialog, Select, etc.
2. **Compose shadcn/ui components** - Combine primitives into feature components
3. **Extend when needed** - Create variants or new components only when shadcn/ui doesn't provide the functionality

**Adding new shadcn/ui components:**
```bash
npx shadcn-ui@latest add button
npx shadcn-ui@latest add card
npx shadcn-ui@latest add dialog
```

**Customizing shadcn/ui components:**
- Modify the generated component files in `components/ui/` directory
- Update Tailwind classes to match design system
- Extend variants using `class-variance-authority` (CVA)
- Never override with excessive custom CSS

### 3. Design System Consistency

Ensure all components use design tokens from the Tailwind configuration rather than arbitrary values. Design tokens maintain visual consistency across the application.

**Design tokens include:**
- **Colors** - Use theme colors (`bg-primary`, `text-foreground`, `border-border`)
- **Spacing** - Use spacing scale (`p-4`, `gap-6`, `space-y-2`)
- **Typography** - Use text utilities (`text-sm`, `font-medium`, `leading-tight`)
- **Border radius** - Use radius scale (`rounded-md`, `rounded-lg`)
- **Shadows** - Use shadow utilities (`shadow-sm`, `shadow-md`)
- **Breakpoints** - Use responsive prefixes (`md:`, `lg:`, `xl:`)

**Good - Uses design tokens:**
```tsx
<div className="rounded-lg border border-border bg-card p-6 shadow-sm">
  <h3 className="text-lg font-semibold">Title</h3>
  <p className="text-sm text-muted-foreground">Description</p>
</div>
```

**Avoid - Arbitrary values:**
```tsx
<div className="rounded-[12px] border border-[#e5e7eb] bg-white p-[24px]" style={{ boxShadow: '0 1px 3px rgba(0,0,0,0.1)' }}>
  <h3 className="text-[18px] font-[600]">Title</h3>
  <p className="text-[14px] text-[#6b7280]">Description</p>
</div>
```

### 4. Single Responsibility Principle

Each component should have a single, well-defined purpose. Components that do too much become difficult to maintain, test, and reuse.

**Identify component responsibility:**
- What is the primary purpose of this component?
- Can it be broken into smaller, focused components?
- Does it manage too many concerns (layout, data fetching, business logic, UI)?

**Component organization:**
```
components/
├── ui/               # shadcn/ui primitives (Button, Card, Dialog)
├── layout/           # Layout components (Header, Sidebar, Container)
├── features/         # Feature-specific components (UserProfile, TaskList)
└── shared/           # Reusable domain components (DataTable, FormField)
```

### 5. Accessibility by Default

Build accessible components from the ground up, not as an afterthought. All components must meet WCAG 2.1 AA standards.

**Core accessibility requirements:**
- Use semantic HTML elements (`<button>`, `<nav>`, `<main>`)
- Provide accessible names (labels, aria-label, aria-labelledby)
- Ensure keyboard navigation works (Tab, Enter, Space, Escape)
- Maintain proper focus management (focus trapping, focus return)
- Use sufficient color contrast (4.5:1 for text, 3:1 for UI components)
- Support screen readers (test with NVDA, JAWS, VoiceOver)

**Note:** Refer to the `frontend-accessibility` skill for comprehensive accessibility guidance when building components.

### 6. TypeScript Type Safety

Use TypeScript to provide type safety and improve developer experience. Well-typed components prevent bugs and provide better autocomplete.

**Component type patterns:**
```tsx
// Define explicit prop types
interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'default' | 'destructive' | 'outline' | 'ghost';
  size?: 'default' | 'sm' | 'lg';
  isLoading?: boolean;
}

// Use generics for flexible components
interface SelectProps<T> {
  value: T;
  onChange: (value: T) => void;
  options: Array<{ label: string; value: T }>;
}

// Extend HTML element props for custom components
interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: 'default' | 'outlined';
}
```

### 7. Performance Optimization

Build components with performance in mind, but avoid premature optimization. Measure before optimizing.

**Performance patterns:**
- Use `React.memo()` for expensive components that receive the same props frequently
- Use `useMemo()` for expensive calculations
- Use `useCallback()` for functions passed to child components
- Lazy load components with `React.lazy()` and `Suspense`
- Virtualize long lists with libraries like `@tanstack/react-virtual`
- Code split large features with dynamic imports

**When to optimize:**
- Component re-renders excessively (use React DevTools Profiler)
- Expensive calculations run on every render
- Large lists cause performance issues
- Bundle size impacts load time

**When not to optimize:**
- Premature optimization without measurement
- Simple components that render quickly
- Components that rarely re-render

## Creating New Components

When building new UI components, follow this systematic workflow:

### Step 1: Identify Component Requirements

Clearly define what the component needs to do:
- What is its primary purpose?
- What props does it need?
- What states does it have (loading, error, success, disabled)?
- What user interactions does it support?
- What accessibility requirements apply?
- How should it respond at different screen sizes?

### Step 2: Check for Existing Solutions

Before creating a new component, verify that it doesn't already exist:
1. **Check shadcn/ui** - Does a primitive component already exist?
2. **Search the codebase** - Is there a similar component that can be reused or extended?
3. **Review the design system** - Are there existing patterns to follow?
4. **Check component library** - Have other developers built something similar?

### Step 3: Choose the Right Foundation

Select the appropriate approach based on the requirements:

**Use shadcn/ui primitive directly:**
- Standard UI elements (buttons, inputs, cards, dialogs)
- Components that match shadcn/ui patterns
- No custom behavior needed

**Compose shadcn/ui components:**
- Feature-specific components built from primitives
- Reusable patterns that combine multiple primitives
- Domain-specific UI (user cards, task items, data tables)

**Create custom component:**
- Unique UI patterns not covered by shadcn/ui
- Complex interactions or animations
- Domain-specific widgets

### Step 4: Structure the Component

Organize component code following React best practices:

**Component file structure:**
```tsx
// 1. Imports
import * as React from 'react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

// 2. Types
interface ComponentProps {
  // Props here
}

// 3. Component implementation
export function Component({ prop1, prop2, ...props }: ComponentProps) {
  // 4. Hooks
  const [state, setState] = React.useState()

  // 5. Event handlers
  const handleClick = () => {
    // Handler logic
  }

  // 6. Render
  return (
    <div>
      {/* JSX here */}
    </div>
  )
}
```

**Naming conventions:**
- Component files: PascalCase (`UserProfile.tsx`, `TaskList.tsx`)
- Component names: PascalCase matching file name
- Props interfaces: `ComponentNameProps`
- Event handlers: `handle` prefix (`handleClick`, `handleSubmit`)
- State variables: descriptive names (`isLoading`, `errorMessage`)

### Step 5: Implement with Tailwind

Style the component using Tailwind utility classes following design system tokens:

**Tailwind best practices:**
```tsx
// Use cn() helper to merge classes conditionally
import { cn } from '@/lib/utils'

<div className={cn(
  // Base styles
  "rounded-lg border border-border bg-card p-6",
  // Conditional styles
  isActive && "border-primary bg-primary/5",
  // Props-based classes
  className
)} />
```

**Responsive design:**
```tsx
<div className="
  grid grid-cols-1           // Mobile: 1 column
  md:grid-cols-2             // Tablet: 2 columns
  lg:grid-cols-3             // Desktop: 3 columns
  gap-4 md:gap-6             // Responsive spacing
" />
```

**Dark mode support:**
```tsx
<div className="
  bg-white dark:bg-slate-900
  text-slate-900 dark:text-slate-50
  border-slate-200 dark:border-slate-800
" />
```

**Note:** Refer to the `frontend-css` skill for comprehensive CSS and Tailwind guidance.

### Step 6: Add Accessibility Features

Ensure the component is accessible from the start:

**Semantic HTML:**
```tsx
// Good - Semantic button
<button onClick={handleClick}>Click me</button>

// Avoid - Non-semantic div
<div onClick={handleClick} role="button" tabIndex={0}>Click me</div>
```

**Accessible labels:**
```tsx
// Form fields with labels
<div>
  <label htmlFor="email" className="text-sm font-medium">
    Email
  </label>
  <Input id="email" type="email" />
</div>

// Icon buttons with accessible names
<Button variant="ghost" size="icon" aria-label="Close dialog">
  <X className="h-4 w-4" />
</Button>
```

**Keyboard navigation:**
```tsx
// Handle keyboard events
<div
  role="button"
  tabIndex={0}
  onClick={handleClick}
  onKeyDown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      handleClick()
    }
  }}
>
  Interactive element
</div>
```

**Focus management:**
```tsx
// Manage focus in dialogs
const dialogRef = React.useRef<HTMLDivElement>(null)

React.useEffect(() => {
  if (isOpen) {
    dialogRef.current?.focus()
  }
}, [isOpen])
```

**Note:** Refer to the `frontend-accessibility` skill for comprehensive accessibility guidance when building components.

### Step 7: Handle Component States

Implement all relevant component states:

**Common states:**
- **Loading** - Data fetching, async operations
- **Error** - Failed operations, validation errors
- **Empty** - No data to display
- **Disabled** - Interaction not allowed
- **Success** - Successful operations

**State implementation:**
```tsx
interface ComponentProps {
  isLoading?: boolean
  error?: string
  data?: DataType[]
}

export function Component({ isLoading, error, data }: ComponentProps) {
  if (isLoading) {
    return <Skeleton />
  }

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    )
  }

  if (!data || data.length === 0) {
    return (
      <div className="text-center text-muted-foreground">
        No data available
      </div>
    )
  }

  return (
    <div>
      {/* Render data */}
    </div>
  )
}
```

### Step 8: Write Component Tests

Test component behavior, not implementation details:

**Testing priorities:**
1. User interactions (clicks, typing, form submission)
2. Accessibility (keyboard navigation, screen reader text)
3. Different states (loading, error, success)
4. Props validation (required props, prop types)
5. Edge cases (empty data, long text, boundary values)

**Example test:**
```tsx
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Button } from './button'

describe('Button', () => {
  it('calls onClick when clicked', async () => {
    const handleClick = vi.fn()
    render(<Button onClick={handleClick}>Click me</Button>)

    await userEvent.click(screen.getByRole('button', { name: /click me/i }))

    expect(handleClick).toHaveBeenCalledTimes(1)
  })

  it('is disabled when disabled prop is true', () => {
    render(<Button disabled>Click me</Button>)

    expect(screen.getByRole('button')).toBeDisabled()
  })
})
```

### Step 9: Document Component Usage

Provide clear documentation for component usage:

**Component documentation should include:**
- Purpose and use cases
- Props interface with descriptions
- Usage examples
- Accessibility considerations
- Visual variants

**Example documentation:**
```tsx
/**
 * A flexible card component for displaying grouped content.
 *
 * @example
 * <Card>
 *   <CardHeader>
 *     <CardTitle>Title</CardTitle>
 *   </CardHeader>
 *   <CardContent>
 *     Content here
 *   </CardContent>
 * </Card>
 */
```

## Component Patterns

Common component patterns to follow when building UI components:

### Compound Components

Use compound components for components with multiple related parts:

```tsx
// Card compound component
export function Card({ children, ...props }: CardProps) {
  return <div {...props}>{children}</div>
}

export function CardHeader({ children, ...props }: CardHeaderProps) {
  return <div {...props}>{children}</div>
}

export function CardContent({ children, ...props }: CardContentProps) {
  return <div {...props}>{children}</div>
}

// Usage
<Card>
  <CardHeader>Header</CardHeader>
  <CardContent>Content</CardContent>
</Card>
```

### Controlled vs Uncontrolled Components

Support both controlled and uncontrolled usage when appropriate:

```tsx
// Support both controlled and uncontrolled
function Input({ value: valueProp, onChange, defaultValue, ...props }: InputProps) {
  const [internalValue, setInternalValue] = React.useState(defaultValue ?? '')

  // Use prop value if controlled, internal state if uncontrolled
  const value = valueProp ?? internalValue
  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!valueProp) {
      setInternalValue(e.target.value)
    }
    onChange?.(e)
  }

  return <input value={value} onChange={handleChange} {...props} />
}
```

### Polymorphic Components

Create components that can render as different HTML elements:

```tsx
type AsProp<C extends React.ElementType> = {
  as?: C
}

type PropsToOmit<C extends React.ElementType, P> = keyof (AsProp<C> & P)

type PolymorphicComponentProp<
  C extends React.ElementType,
  Props = {}
> = React.PropsWithChildren<Props & AsProp<C>> &
  Omit<React.ComponentPropsWithoutRef<C>, PropsToOmit<C, Props>>

export function Text<C extends React.ElementType = 'span'>({
  as,
  children,
  ...props
}: PolymorphicComponentProp<C>) {
  const Component = as || 'span'
  return <Component {...props}>{children}</Component>
}

// Usage
<Text as="h1">Heading</Text>
<Text as="p">Paragraph</Text>
```

### Render Props

Use render props for flexible component customization:

```tsx
interface DataListProps<T> {
  data: T[]
  renderItem: (item: T) => React.ReactNode
  renderEmpty?: () => React.ReactNode
}

export function DataList<T>({ data, renderItem, renderEmpty }: DataListProps<T>) {
  if (data.length === 0) {
    return renderEmpty ? renderEmpty() : <div>No data</div>
  }

  return (
    <ul>
      {data.map((item, index) => (
        <li key={index}>{renderItem(item)}</li>
      ))}
    </ul>
  )
}

// Usage
<DataList
  data={users}
  renderItem={(user) => <UserCard user={user} />}
  renderEmpty={() => <EmptyState />}
/>
```

## Reviewing Components

When reviewing component code for adherence to standards:

### Step 1: Check Component Structure

Verify the component follows structural best practices:
- [ ] Single responsibility - Component has one clear purpose
- [ ] Proper organization - Imports, types, hooks, handlers, render
- [ ] Naming conventions - PascalCase components, descriptive names
- [ ] File location - Component in appropriate directory
- [ ] Dependencies - Minimal external dependencies

### Step 2: Verify shadcn/ui Integration

Ensure proper use of shadcn/ui components:
- [ ] Uses shadcn/ui primitives when available
- [ ] Follows composition patterns, not complex configuration
- [ ] Customizes through Tailwind classes, not CSS overrides
- [ ] Extends variants using CVA when needed
- [ ] No unnecessary custom components

### Step 3: Assess Design System Consistency

Check adherence to design system tokens:
- [ ] Colors use theme tokens (`bg-primary`, `text-foreground`)
- [ ] Spacing uses spacing scale (`p-4`, `gap-6`)
- [ ] Typography uses text utilities (`text-sm`, `font-medium`)
- [ ] No arbitrary values without justification
- [ ] Responsive breakpoints use standard prefixes

### Step 4: Evaluate TypeScript Usage

Review TypeScript implementation:
- [ ] Props interface is well-defined
- [ ] Extends appropriate HTML element types
- [ ] No `any` types without justification
- [ ] Generics used appropriately for flexible components
- [ ] Type imports use `import type` for type-only imports

### Step 5: Check Accessibility

Verify accessibility implementation:
- [ ] Semantic HTML elements used
- [ ] Accessible names provided (labels, aria-label)
- [ ] Keyboard navigation works
- [ ] Focus management handled correctly
- [ ] Color contrast meets WCAG AA (4.5:1 text, 3:1 UI)
- [ ] Screen reader experience is clear

Refer to the `frontend-accessibility` skill for comprehensive accessibility review.

### Step 6: Test Component States

Verify all states are handled:
- [ ] Loading state implemented and accessible
- [ ] Error state provides clear feedback
- [ ] Empty state guides user action
- [ ] Disabled state prevents interaction
- [ ] Success state confirms completion

### Step 7: Review Performance

Check for performance issues:
- [ ] No unnecessary re-renders
- [ ] Expensive calculations use `useMemo()`
- [ ] Event handlers use `useCallback()` when passed to children
- [ ] Large lists are virtualized if needed
- [ ] Code splitting used for large components

### Step 8: Provide Constructive Feedback

When issues are found:
1. Identify what doesn't align with standards
2. Explain why it matters (maintainability, accessibility, performance)
3. Suggest specific improvements with examples
4. Reference the relevant principle or pattern
5. Acknowledge what was done well

## Common Component Examples

Reference implementations for common component patterns:

### Form Field Component

A reusable form field that integrates label, input, and error message:

```tsx
interface FormFieldProps {
  label: string
  error?: string
  required?: boolean
  children: React.ReactNode
}

export function FormField({ label, error, required, children }: FormFieldProps) {
  const id = React.useId()
  const errorId = `${id}-error`

  return (
    <div className="space-y-2">
      <label htmlFor={id} className="text-sm font-medium">
        {label}
        {required && <span className="text-destructive ml-1">*</span>}
      </label>
      {React.cloneElement(children as React.ReactElement, {
        id,
        'aria-invalid': !!error,
        'aria-describedby': error ? errorId : undefined,
      })}
      {error && (
        <p id={errorId} className="text-sm text-destructive">
          {error}
        </p>
      )}
    </div>
  )
}

// Usage
<FormField label="Email" error={errors.email} required>
  <Input type="email" />
</FormField>
```

### Data Table Component

A flexible data table using composition:

```tsx
interface DataTableProps<T> {
  data: T[]
  columns: Array<{
    header: string
    accessorKey: keyof T
    cell?: (value: T[keyof T]) => React.ReactNode
  }>
}

export function DataTable<T>({ data, columns }: DataTableProps<T>) {
  return (
    <div className="rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            {columns.map((column) => (
              <TableHead key={String(column.accessorKey)}>
                {column.header}
              </TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          {data.map((row, index) => (
            <TableRow key={index}>
              {columns.map((column) => (
                <TableCell key={String(column.accessorKey)}>
                  {column.cell
                    ? column.cell(row[column.accessorKey])
                    : String(row[column.accessorKey])}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

// Usage
<DataTable
  data={users}
  columns={[
    { header: 'Name', accessorKey: 'name' },
    { header: 'Email', accessorKey: 'email' },
    {
      header: 'Status',
      accessorKey: 'status',
      cell: (value) => <Badge>{value}</Badge>,
    },
  ]}
/>
```

### Modal Dialog Component

An accessible modal dialog with focus management:

```tsx
interface ModalProps {
  isOpen: boolean
  onClose: () => void
  title: string
  children: React.ReactNode
}

export function Modal({ isOpen, onClose, title, children }: ModalProps) {
  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        {children}
      </DialogContent>
    </Dialog>
  )
}

// Usage
<Modal
  isOpen={isModalOpen}
  onClose={() => setIsModalOpen(false)}
  title="Confirm Action"
>
  <p>Are you sure you want to continue?</p>
  <DialogFooter>
    <Button variant="outline" onClick={() => setIsModalOpen(false)}>
      Cancel
    </Button>
    <Button onClick={handleConfirm}>Confirm</Button>
  </DialogFooter>
</Modal>
```

## Summary

When creating UI components:

1. **Compose over configure** - Build flexible components through composition
2. **Leverage shadcn/ui** - Use primitives and compose them into feature components
3. **Follow design system** - Use Tailwind tokens, not arbitrary values
4. **Single responsibility** - Each component has one clear purpose
5. **Accessibility first** - Build accessible components from the start (refer to frontend-accessibility skill)
6. **Type safety** - Use TypeScript for better developer experience
7. **Optimize wisely** - Measure before optimizing, avoid premature optimization
8. **Test behavior** - Test user interactions and accessibility, not implementation
9. **Document usage** - Provide clear examples and prop descriptions
10. **Review thoroughly** - Ensure consistency, accessibility, and performance

Building consistent, accessible, and maintainable UI components requires following established patterns and design system standards. These practices ensure a high-quality user experience across the application.
