import { z } from 'zod';

const trimmedString = (max: number, field: string) =>
  z
    .string()
    .min(1, `${field} is required`)
    .max(max, `${field} must be at most ${max} characters`)
    .refine((s) => s === s.trim(), `${field} must not have leading or trailing whitespace`);

const configJson = z
  .string()
  .optional()
  .default('{}')
  .superRefine((raw, ctx) => {
    const text = raw.trim();
    if (text.length === 0) return;
    try {
      const parsed = JSON.parse(text);
      if (
        parsed === null ||
        typeof parsed !== 'object' ||
        Array.isArray(parsed)
      ) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Config must be a JSON object',
        });
        return;
      }
      const bytes = new TextEncoder().encode(JSON.stringify(parsed)).length;
      if (bytes > 16 * 1024) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Config must be 16 KB or smaller',
        });
      }
    } catch {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'Config must be valid JSON',
      });
    }
  });

export const constraintOperators = [
  'EQ',
  'NEQ',
  'GT',
  'GTE',
  'LT',
  'LTE',
  'IN',
  'NOT_IN',
] as const;

export const operatorLabels: Record<
  (typeof constraintOperators)[number],
  string
> = {
  EQ: 'Equals',
  NEQ: "Doesn't equal",
  GT: 'Greater than',
  GTE: 'Greater than or equal',
  LT: 'Less than',
  LTE: 'Less than or equal',
  IN: 'Is one of',
  NOT_IN: "Isn't one of",
};

export const constraintSchema = z.object({
  property: trimmedString(256, 'Property'),
  operator: z.enum(constraintOperators),
  // Raw value as string; will be parsed based on operator at submit time.
  value: z.string().min(1, 'Value is required'),
});

export const distributionSchema = z.object({
  variantKey: z.string().min(1, 'Pick a variant'),
  percent: z.coerce.number().int().min(0).max(100),
});

export const segmentSchema = z.object({
  priority: z.coerce.number().int().min(0, 'Priority must be >= 0'),
  rolloutPercent: z.coerce.number().int().min(0).max(100),
  constraints: z.array(constraintSchema).max(64),
  distributions: z
    .array(distributionSchema)
    .min(1, 'At least one distribution')
    .max(64),
});

export const variantSchema = z.object({
  key: trimmedString(256, 'Variant key'),
  isControl: z.boolean(),
  config: configJson,
});

export const experimentFormSchema = z
  .object({
    key: trimmedString(256, 'Key'),
    description: z
      .string()
      .max(4096, 'Description must be at most 4096 characters')
      .optional()
      .transform((s) => (s && s.trim().length > 0 ? s : undefined)),
    primaryMetric: trimmedString(256, 'Primary metric'),
    variants: z.array(variantSchema).min(1, 'At least one variant').max(64),
    segments: z.array(segmentSchema).min(1, 'At least one segment').max(64),
  })
  .superRefine((data, ctx) => {
    // Exactly one control variant.
    const controlCount = data.variants.filter((v) => v.isControl).length;
    if (controlCount !== 1) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['variants'],
        message: 'Exactly one variant must be marked as control',
      });
    }

    // Unique variant keys.
    const variantKeys = new Set<string>();
    data.variants.forEach((v, idx) => {
      if (variantKeys.has(v.key)) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['variants', idx, 'key'],
          message: 'Variant keys must be unique',
        });
      }
      variantKeys.add(v.key);
    });

    // Unique segment priorities; distributions valid and summing to 100.
    const priorities = new Set<number>();
    data.segments.forEach((seg, sIdx) => {
      if (priorities.has(seg.priority)) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['segments', sIdx, 'priority'],
          message: 'Segment priorities must be unique',
        });
      }
      priorities.add(seg.priority);

      const sum = seg.distributions.reduce((acc, d) => acc + d.percent, 0);
      if (sum !== 100) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['segments', sIdx, 'distributions'],
          message: `Distribution percents must sum to 100 (got ${sum})`,
        });
      }

      const seenVariants = new Set<string>();
      seg.distributions.forEach((d, dIdx) => {
        if (!variantKeys.has(d.variantKey)) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['segments', sIdx, 'distributions', dIdx, 'variantKey'],
            message: 'Unknown variant key',
          });
        }
        if (seenVariants.has(d.variantKey)) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['segments', sIdx, 'distributions', dIdx, 'variantKey'],
            message: 'Duplicate variant in distributions',
          });
        }
        seenVariants.add(d.variantKey);
      });

      seg.constraints.forEach((c, cIdx) => {
        const err = parseConstraintValueError(c.operator, c.value);
        if (err) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['segments', sIdx, 'constraints', cIdx, 'value'],
            message: err,
          });
        }
      });
    });
  });

export type ExperimentFormValues = z.infer<typeof experimentFormSchema>;

/**
 * Parses a constraint value according to its operator. Returns either the
 * parsed value (suitable for sending to the API) or a validation error
 * message.
 */
export function parseConstraintValue(
  operator: (typeof constraintOperators)[number],
  raw: string,
): { ok: true; value: string | number | boolean | Array<string | number | boolean> } | { ok: false; error: string } {
  const text = raw.trim();
  if (text.length === 0) return { ok: false, error: 'Value is required' };

  const parseScalar = (s: string): string | number | boolean => {
    if (s === 'true') return true;
    if (s === 'false') return false;
    if (s !== '' && !Number.isNaN(Number(s))) return Number(s);
    return s;
  };

  if (operator === 'IN' || operator === 'NOT_IN') {
    // Try JSON array first, else comma-separated.
    let items: Array<string | number | boolean>;
    if (text.startsWith('[')) {
      try {
        const parsed = JSON.parse(text);
        if (!Array.isArray(parsed) || parsed.length === 0) {
          return { ok: false, error: 'Must be a non-empty JSON array' };
        }
        if (
          !parsed.every(
            (x) =>
              typeof x === 'string' ||
              typeof x === 'number' ||
              typeof x === 'boolean',
          )
        ) {
          return {
            ok: false,
            error: 'Array items must be strings, numbers, or booleans',
          };
        }
        items = parsed;
      } catch {
        return { ok: false, error: 'Invalid JSON array' };
      }
    } else {
      items = text
        .split(',')
        .map((s) => s.trim())
        .filter((s) => s.length > 0)
        .map(parseScalar);
      if (items.length === 0) {
        return { ok: false, error: 'At least one value required' };
      }
    }
    return { ok: true, value: items };
  }

  if (operator === 'GT' || operator === 'GTE' || operator === 'LT' || operator === 'LTE') {
    const n = Number(text);
    if (Number.isNaN(n)) {
      return { ok: false, error: 'Must be a number for this operator' };
    }
    return { ok: true, value: n };
  }

  // EQ / NEQ: scalar.
  return { ok: true, value: parseScalar(text) };
}

function parseConstraintValueError(
  operator: (typeof constraintOperators)[number],
  raw: string,
): string | null {
  const res = parseConstraintValue(operator, raw);
  return res.ok ? null : res.error;
}
