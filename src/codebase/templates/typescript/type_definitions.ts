// @LITE_DESC TypeScript type definitions with interfaces, generics, and utility types
// @LITE_SCENE Comprehensive TypeScript patterns including advanced types and type manipulation
// @LITE_TAGS typescript,types,interface,generics,definitions

// Basic interfaces and type aliases
interface User {
  id: number;
  name: string;
  email: string;
  age?: number; // Optional property
}

type UserId = number | string;
type UserStatus = 'active' | 'inactive' | 'pending';

// Interface extension
interface AdminUser extends User {
  permissions: string[];
  role: 'admin' | 'superadmin';
}

// Type aliases with union types
type Result<T> =
  | { success: true; data: T }
  | { success: false; error: string };

// Generic interfaces
interface Repository<T, K = number> {
  findById(id: K): Promise<T | null>;
  findAll(): Promise<T[]>;
  create(entity: Omit<T, 'id'>): Promise<T>;
  update(id: K, updates: Partial<T>): Promise<T>;
  delete(id: K): Promise<boolean>;
}

// Specific repository implementation
interface UserRepository extends Repository<User> {
  findByEmail(email: string): Promise<User | null>;
  findByStatus(status: UserStatus): Promise<User[]>;
}

// Utility types
type CreateUserDto = Omit<User, 'id'>; // Remove id
type UpdateUserDto = Partial<CreateUserDto>; // Make all properties optional
type UserKeys = keyof User; // Get keys as union type
type UserValues = User[keyof User]; // Get values as union type

// Conditional types
type NonNullable<T> = T extends null | undefined ? never : T;
type IsArray<T> = T extends any[] ? true : false;

// Mapped types
type ReadonlyUser = {
  readonly [K in keyof User]: User[K];
};

type NullableUser = {
  [K in keyof User]?: User[K] | null;
};

// Template literal types
type EventName = `on${Capitalize<string>}`;
type UserEvent = `user${'Created' | 'Updated' | 'Deleted'}`;

// Function types
type Callback<T> = (data: T) => void;
type AsyncFunction<T, R> = (input: T) => Promise<R>;

// Generic functions
function identity<T>(arg: T): T {
  return arg;
}

function first<T>(array: T[]): T | undefined {
  return array[0];
}

function map<T, U>(array: T[], mapper: (item: T) => U): U[] {
  return array.map(mapper);
}

// Generic constraints
function logLength<T extends { length: number }>(arg: T): void {
  console.log(arg.length);
}

// Multiple type parameters with constraints
function merge<T extends object, U extends object>(obj1: T, obj2: U): T & U {
  return { ...obj1, ...obj2 };
}

// Discriminated unions
interface Shape {
  kind: 'circle' | 'square' | 'rectangle';
}

interface Circle extends Shape {
  kind: 'circle';
  radius: number;
}

interface Square extends Shape {
  kind: 'square';
  side: number;
}

interface Rectangle extends Shape {
  kind: 'rectangle';
  width: number;
  height: number;
}

type AnyShape = Circle | Square | Rectangle;

function calculateArea(shape: AnyShape): number {
  switch (shape.kind) {
    case 'circle':
      return Math.PI * shape.radius ** 2;
    case 'square':
      return shape.side ** 2;
    case 'rectangle':
      return shape.width * shape.height;
    default:
      const exhaustiveCheck: never = shape;
      return exhaustiveCheck;
  }
}

// Branded types for type safety
type UserId2 = string & { readonly __brand: unique symbol };
type EmailAddress = string & { readonly __brand: unique symbol };

function createUserId(id: string): UserId2 {
  return id as UserId2;
}

function createEmail(email: string): EmailAddress {
  if (!email.includes('@')) {
    throw new Error('Invalid email format');
  }
  return email as EmailAddress;
}

// Type guards
function isCircle(shape: Shape): shape is Circle {
  return shape.kind === 'circle';
}

function isAdminUser(user: User): user is AdminUser {
  return 'role' in user && (user.role === 'admin' || user.role === 'superadmin');
}

// Generic type guards
function isArray<T>(value: unknown): value is T[] {
  return Array.isArray(value);
}

// Utility type implementations
type Partial<T> = {
  [P in keyof T]?: T[P];
};

type Required<T> = {
  [P in keyof T]-?: T[P];
};

type Readonly<T> = {
  readonly [P in keyof T]: T[P];
};

type Pick<T, K extends keyof T> = {
  [P in K]: T[P];
};

type Omit<T, K extends keyof T> = Pick<T, Exclude<keyof T, K>>;

// Advanced utility types
type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P];
};

type DeepReadonly<T> = {
  readonly [P in keyof T]: T[P] extends object ? DeepReadonly<T[P]> : T[P];
};

// ReturnType and Parameters utility types
type AsyncResult = ReturnType<typeof calculateArea>; // number
type FunctionParams = Parameters<typeof Math.max>; // number[]

// Declaration merging
interface Document {
  title: string;
}

interface Document {
  content: string;
  metadata?: Record<string, unknown>;
}

// Interface with callable signature
interface Counter {
  (start?: number): void;
  count: number;
  reset(): void;
}

function createCounter(): Counter {
  const counter = ((start = 0) => {
    counter.count = start;
  }) as Counter;

  counter.count = 0;
  counter.reset = () => {
    counter.count = 0;
  };

  return counter;
}

// Generic classes
class Stack<T> {
  private items: T[] = [];

  push(item: T): void {
    this.items.push(item);
  }

  pop(): T | undefined {
    return this.items.pop();
  }

  peek(): T | undefined {
    return this.items[this.items.length - 1];
  }

  isEmpty(): boolean {
    return this.items.length === 0;
  }

  size(): number {
    return this.items.length;
  }
}

// Generic with multiple constraints
interface Serializable {
  toJSON(): string;
}

function serialize<T extends Serializable>(obj: T): string {
  return obj.toJSON();
}

// Type inference from function parameters
function createPair<S, T>(first: S, second: T): [S, T] {
  return [first, second];
}

// Conditional type inference
type UnwrapPromise<T> = T extends Promise<infer U> ? U : T;
type UnwrapArray<T> = T extends (infer U)[] ? U : T;

// Example usage
type Data = UnwrapPromise<Promise<string>>; // string
type Item = UnwrapArray<number[]>; // number

// Indexed access types
type UserName = User['name']; // string
type UserOptionalFields = User['age' | 'email']; // number | string

// Keyof with generics
function getProperty<T, K extends keyof T>(obj: T, key: K): T[K] {
  return obj[key];
}

// Record type
type UserRoles = Record<string, string[]>;

const roles: UserRoles = {
  admin: ['read', 'write', 'delete'],
  user: ['read'],
  guest: []
};

// Map types
type ReadonlyFields<T, K extends keyof T> = Readonly<Pick<T, K>> & Omit<T, K>;

type ReadonlyUserFields = ReadonlyFields<User, 'id' | 'email'>;
