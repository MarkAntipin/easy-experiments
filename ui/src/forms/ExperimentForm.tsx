import { useEffect, useMemo } from 'react';
import { useForm, useFieldArray, Controller } from 'react-hook-form';
import type { UseFormReturn } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { AlertTriangle, Plus, Trash2 } from 'lucide-react';
import { Button } from '@/components/Button';
import { Field, Input, Select, Textarea } from '@/components/Input';
import {
  PriorityStack,
  SegmentDistribution,
  VariantPreview,
} from '@/components/teaching';
import { cn } from '@/lib/cn';
import {
  constraintOperators,
  experimentFormSchema,
  operatorLabels,
  parseConstraintValue,
  type ExperimentFormValues,
} from './experimentSchema';
import type {
  Constraint,
  ConstraintValue,
  CreateExperimentRequest,
  ExperimentDetail,
  Segment,
  Variant,
} from '@/api/types';

export interface ExperimentFormProps {
  initial?: ExperimentDetail;
  mode: 'create' | 'edit';
  submitting: boolean;
  submitLabel: string;
  onSubmit: (payload: CreateExperimentRequest) => void | Promise<void>;
  /**
   * If true, "key", "variants", and "segments" cannot be changed. Used when
   * editing an experiment that is running/stopped — only description and
   * primaryMetric edits are forwarded to the API in that case.
   */
  locked?: boolean;
}

function emptyDefaults(): ExperimentFormValues {
  return {
    key: '',
    description: undefined,
    primaryMetric: '',
    variants: [
      { key: 'control', isControl: true, config: '{}' },
      { key: 'treatment', isControl: false, config: '{}' },
    ],
    segments: [
      {
        priority: 0,
        rolloutPercent: 100,
        constraints: [],
        distributions: [
          { variantKey: 'control', percent: 50 },
          { variantKey: 'treatment', percent: 50 },
        ],
      },
    ],
  };
}

function stringifyConstraintValue(value: ConstraintValue): string {
  if (Array.isArray(value)) {
    return value.map((v) => String(v)).join(', ');
  }
  return String(value);
}

function detailToValues(d: ExperimentDetail): ExperimentFormValues {
  return {
    key: d.key,
    description: d.description ?? undefined,
    primaryMetric: d.primaryMetric,
    variants: d.variants.map((v) => ({
      key: v.key,
      isControl: v.isControl,
      config: JSON.stringify(v.config ?? {}, null, 2),
    })),
    segments: d.segments.map((s) => ({
      priority: s.priority,
      rolloutPercent: s.rolloutPercent,
      constraints: s.constraints.map((c) => ({
        property: c.property,
        operator: c.operator,
        value: stringifyConstraintValue(c.value),
      })),
      distributions: s.distributions.map((dist) => ({
        variantKey: dist.variantKey,
        percent: dist.percent,
      })),
    })),
  };
}

function valuesToPayload(v: ExperimentFormValues): CreateExperimentRequest {
  const variants: Variant[] = v.variants.map((vv) => {
    const raw = (vv.config ?? '').trim();
    const config = raw.length === 0 ? {} : (JSON.parse(raw) as Record<string, unknown>);
    return {
      key: vv.key,
      isControl: vv.isControl,
      config,
    };
  });

  const segments: Segment[] = v.segments.map((seg) => ({
    priority: seg.priority,
    rolloutPercent: seg.rolloutPercent,
    distributions: seg.distributions.map((d) => ({
      variantKey: d.variantKey,
      percent: d.percent,
    })),
    constraints: seg.constraints.map<Constraint>((c) => {
      const parsed = parseConstraintValue(c.operator, c.value);
      if (!parsed.ok) {
        // Schema validation guards this path; this is defensive only.
        throw new Error(`Invalid constraint value: ${parsed.error}`);
      }
      return {
        property: c.property,
        operator: c.operator,
        value: parsed.value,
      };
    }),
  }));

  return {
    key: v.key,
    description: v.description ?? null,
    primaryMetric: v.primaryMetric,
    variants,
    segments,
  };
}

export function ExperimentForm({
  initial,
  mode,
  submitting,
  submitLabel,
  onSubmit,
  locked,
}: ExperimentFormProps) {
  const defaultValues = useMemo<ExperimentFormValues>(
    () => (initial ? detailToValues(initial) : emptyDefaults()),
    [initial],
  );

  const form = useForm<ExperimentFormValues>({
    resolver: zodResolver(experimentFormSchema),
    defaultValues,
    mode: 'onBlur',
  });

  useEffect(() => {
    form.reset(defaultValues);
  }, [defaultValues, form]);

  const {
    register,
    control,
    handleSubmit,
    watch,
    setValue,
    formState: { errors },
  } = form;

  const variantsArray = useFieldArray({ control, name: 'variants' });
  const segmentsArray = useFieldArray({ control, name: 'segments' });
  const watchedVariants = watch('variants');

  return (
    <form
      onSubmit={handleSubmit((values) => onSubmit(valuesToPayload(values)))}
      className="flex flex-col gap-6"
    >
      <section className="rounded-lg border border-slate-200 bg-white p-5">
        <h2 className="mb-1 text-sm font-semibold text-slate-900">Basics</h2>
        <p className="mb-4 text-xs text-slate-500">
          A short identifier for this experiment and the metric you&rsquo;ll
          watch to decide which variant wins.
        </p>
        <div className="grid gap-4 sm:grid-cols-2">
          <Field
            label="Key"
            htmlFor="key"
            required
            error={errors.key?.message}
            hint="Unique identifier used at evaluation time."
          >
            <Input
              id="key"
              placeholder="homepage_cta"
              disabled={locked || mode === 'edit'}
              {...register('key')}
            />
          </Field>
          <Field
            label="Primary metric"
            htmlFor="primaryMetric"
            required
            error={errors.primaryMetric?.message}
            hint="Identifier for the outcome you're optimizing — track it in your analytics so you can compare variants."
          >
            <Input
              id="primaryMetric"
              placeholder="signup_conversion"
              disabled={locked}
              {...register('primaryMetric')}
            />
          </Field>
          <Field
            label="Description"
            htmlFor="description"
            error={errors.description?.message}
            className="sm:col-span-2"
          >
            <Textarea
              id="description"
              placeholder="Optional context for teammates."
              className="font-sans"
              {...register('description')}
            />
          </Field>
        </div>
      </section>

      <VariantsSection
        form={form}
        array={variantsArray}
        locked={locked}
        onControlChange={(idx) => {
          // Exactly one control: unset other flags when a new control is picked.
          watchedVariants.forEach((_, i) => {
            setValue(`variants.${i}.isControl`, i === idx, {
              shouldDirty: true,
              shouldValidate: true,
            });
          });
        }}
      />

      <SegmentsSection
        form={form}
        array={segmentsArray}
        variantKeys={watchedVariants.map((v) => v.key).filter(Boolean)}
        locked={locked}
      />

      <div className="flex items-center justify-end gap-2">
        <Button type="submit" loading={submitting}>
          {submitLabel}
        </Button>
      </div>
    </form>
  );
}

// ---------------- Variants ----------------

function VariantsSection({
  form,
  array,
  locked,
  onControlChange,
}: {
  form: UseFormReturn<ExperimentFormValues>;
  array: ReturnType<typeof useFieldArray<ExperimentFormValues, 'variants'>>;
  locked?: boolean;
  onControlChange: (index: number) => void;
}) {
  const { register, formState: { errors } } = form;
  const rootError = errors.variants?.message ?? errors.variants?.root?.message;

  return (
    <section className="rounded-lg border border-slate-200 bg-white p-5">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h2 className="text-sm font-semibold text-slate-900">Variants</h2>
          <p className="text-xs text-slate-500">
            The different versions you want to test. Mark one as the control —
            the others will be compared against it.
          </p>
        </div>
        {!locked ? (
          <Button
            size="sm"
            variant="secondary"
            onClick={() =>
              array.append({ key: '', isControl: false, config: '{}' })
            }
          >
            <Plus className="h-4 w-4" />
            Add variant
          </Button>
        ) : null}
      </div>

      {rootError ? (
        <p className="mb-3 text-xs text-red-600">{rootError}</p>
      ) : null}

      <div className="flex flex-col gap-3">
        {array.fields.map((field, idx) => {
          const err = errors.variants?.[idx];
          const liveKey = form.watch(`variants.${idx}.key`) ?? '';
          const liveIsControl = !!form.watch(`variants.${idx}.isControl`);
          const liveConfig = form.watch(`variants.${idx}.config`) ?? '{}';
          return (
            <div
              key={field.id}
              className="grid gap-3 rounded-md border border-slate-200 p-3 sm:grid-cols-[1fr_auto_1fr_auto]"
            >
              <Field
                label="Key"
                error={err?.key?.message}
                htmlFor={`variant-${idx}-key`}
              >
                <Input
                  id={`variant-${idx}-key`}
                  placeholder="control"
                  disabled={locked}
                  {...register(`variants.${idx}.key`)}
                />
              </Field>
              <div className="flex flex-col gap-1">
                <span className="text-sm font-medium text-slate-700">Control</span>
                <label className="mt-1 inline-flex h-10 items-center gap-2">
                  <input
                    type="radio"
                    name="control-variant"
                    disabled={locked}
                    checked={liveIsControl}
                    onChange={() => onControlChange(idx)}
                    className="h-4 w-4 border-slate-300 text-brand-600 focus:ring-brand-500"
                  />
                  <span className="text-xs text-slate-500">Use as control</span>
                </label>
              </div>
              <Field
                label="Config (JSON)"
                error={err?.config?.message}
                htmlFor={`variant-${idx}-cfg`}
                hint="Optional payload returned to your code when this variant is picked."
              >
                <Textarea
                  id={`variant-${idx}-cfg`}
                  rows={2}
                  disabled={locked}
                  {...register(`variants.${idx}.config`)}
                />
              </Field>
              <div className="flex items-start pt-6">
                {!locked && array.fields.length > 1 ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => array.remove(idx)}
                    aria-label="Remove variant"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                ) : null}
              </div>
              <div className="sm:col-span-4">
                <VariantPreview
                  variantKey={liveKey}
                  isControl={liveIsControl}
                  configRaw={liveConfig}
                  variantIndex={idx}
                />
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}

// ---------------- Segments ----------------

function SegmentsSection({
  form,
  array,
  variantKeys,
  locked,
}: {
  form: UseFormReturn<ExperimentFormValues>;
  array: ReturnType<typeof useFieldArray<ExperimentFormValues, 'segments'>>;
  variantKeys: string[];
  locked?: boolean;
}) {
  const { formState: { errors } } = form;
  const rootError = errors.segments?.message ?? errors.segments?.root?.message;

  return (
    <section className="rounded-lg border border-slate-200 bg-white p-5">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h2 className="text-sm font-semibold text-slate-900">Segments</h2>
          <p className="text-xs text-slate-500">
            Decide who sees this experiment and how they&rsquo;re split. Lower
            priority numbers are checked first; the first matching segment
            wins.
          </p>
        </div>
        {!locked ? (
          <Button
            size="sm"
            variant="secondary"
            onClick={() => {
              const nextPriority =
                array.fields.length > 0
                  ? Math.max(
                      ...form
                        .getValues('segments')
                        .map((s) => Number(s.priority) || 0),
                    ) + 1
                  : 0;
              array.append({
                priority: nextPriority,
                rolloutPercent: 100,
                constraints: [],
                distributions:
                  variantKeys.length > 0
                    ? [
                        {
                          variantKey: variantKeys[0] ?? '',
                          percent: 100,
                        },
                      ]
                    : [],
              });
            }}
          >
            <Plus className="h-4 w-4" />
            Add segment
          </Button>
        ) : null}
      </div>

      {rootError ? (
        <p className="mb-3 text-xs text-red-600">{rootError}</p>
      ) : null}

      {array.fields.length >= 2 ? (
        <div className="mb-3">
          <PriorityStack
            variantKeys={variantKeys}
            segments={array.fields.map((field, idx) => {
              const seg = form.watch(`segments.${idx}`);
              return {
                fieldId: field.id,
                priority: Number(seg?.priority ?? 0) || 0,
                rolloutPercent: Number(seg?.rolloutPercent ?? 0) || 0,
                constraintCount: seg?.constraints?.length ?? 0,
                distributions:
                  seg?.distributions?.map((d) => ({
                    variantKey: d.variantKey ?? '',
                    percent: Number(d.percent ?? 0) || 0,
                  })) ?? [],
                positionLabel: idx + 1,
              };
            })}
          />
        </div>
      ) : null}

      <div className="flex flex-col gap-4">
        {array.fields.map((field, idx) => (
          <SegmentCard
            key={field.id}
            index={idx}
            form={form}
            variantKeys={variantKeys}
            locked={locked}
            onRemove={() => array.remove(idx)}
            canRemove={array.fields.length > 1}
          />
        ))}
      </div>
    </section>
  );
}

function SegmentCard({
  index,
  form,
  variantKeys,
  locked,
  onRemove,
  canRemove,
}: {
  index: number;
  form: UseFormReturn<ExperimentFormValues>;
  variantKeys: string[];
  locked?: boolean;
  onRemove: () => void;
  canRemove: boolean;
}) {
  const { register, control, formState: { errors } } = form;
  const err = errors.segments?.[index];

  const constraintArray = useFieldArray({
    control,
    name: `segments.${index}.constraints`,
  });
  const distArray = useFieldArray({
    control,
    name: `segments.${index}.distributions`,
  });

  const distributions = form.watch(`segments.${index}.distributions`);
  const sum = (distributions ?? []).reduce(
    (acc, d) => acc + (Number(d?.percent) || 0),
    0,
  );
  const overshoot = Math.max(0, sum - 100);
  const undershoot = Math.max(0, 100 - sum);

  return (
    <div className="rounded-md border border-slate-200 bg-slate-50/40 p-4">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-sm font-medium text-slate-800">
          Segment #{index + 1}
        </h3>
        {!locked && canRemove ? (
          <Button
            size="sm"
            variant="ghost"
            onClick={onRemove}
            aria-label="Remove segment"
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        ) : null}
      </div>

      <div className="mb-3">
        <SegmentDistribution
          distributions={(distributions ?? []).map((d) => ({
            variantKey: d?.variantKey ?? '',
            percent: Number(d?.percent) || 0,
          }))}
          variantKeys={variantKeys}
        />
      </div>

      <div className="mb-4 grid gap-3 sm:grid-cols-2">
        <Field
          label="Priority"
          required
          error={err?.priority?.message}
          hint="Lower values are checked first."
        >
          <Input
            type="number"
            min={0}
            disabled={locked}
            {...register(`segments.${index}.priority`, { valueAsNumber: true })}
          />
        </Field>
        <Field
          label="Rollout %"
          required
          error={err?.rolloutPercent?.message}
          hint="Portion of eligible users bucketed into this segment."
        >
          <Input
            type="number"
            min={0}
            max={100}
            disabled={locked}
            {...register(`segments.${index}.rolloutPercent`, {
              valueAsNumber: true,
            })}
          />
        </Field>
      </div>

      <div className="mb-4">
        <div className="mb-2 flex items-center justify-between">
          <div>
            <span className="text-xs font-semibold uppercase tracking-wide text-slate-500">
              Constraints
            </span>
            <p className="mt-0.5 text-[11px] text-slate-500">
              Filter who matches this segment. Property names come from the{' '}
              <code className="rounded bg-slate-100 px-1 font-mono">
                properties
              </code>{' '}
              object you pass to evaluate (e.g. country, plan_tier, deviceType).
            </p>
          </div>
          {!locked ? (
            <Button
              size="sm"
              variant="ghost"
              onClick={() =>
                constraintArray.append({
                  property: '',
                  operator: 'EQ',
                  value: '',
                })
              }
            >
              <Plus className="h-4 w-4" />
              Add constraint
            </Button>
          ) : null}
        </div>
        {constraintArray.fields.length === 0 ? (
          <p className="text-xs text-slate-500">
            No constraints — all users match this segment.
          </p>
        ) : (
          <div className="flex flex-col gap-2">
            {constraintArray.fields.map((cField, cIdx) => {
              const cErr = err?.constraints?.[cIdx];
              return (
                <div
                  key={cField.id}
                  className="grid grid-cols-1 gap-2 rounded border border-slate-200 bg-white p-2 sm:grid-cols-[1fr_180px_1fr_auto]"
                >
                  <Input
                    placeholder="country"
                    disabled={locked}
                    aria-invalid={!!cErr?.property}
                    {...register(`segments.${index}.constraints.${cIdx}.property`)}
                  />
                  <Controller
                    control={control}
                    name={`segments.${index}.constraints.${cIdx}.operator`}
                    render={({ field }) => (
                      <Select {...field} disabled={locked}>
                        {constraintOperators.map((op) => (
                          <option key={op} value={op}>
                            {operatorLabels[op]}
                          </option>
                        ))}
                      </Select>
                    )}
                  />
                  <Input
                    placeholder='value (use "a, b, c" for "Is one of")'
                    disabled={locked}
                    aria-invalid={!!cErr?.value}
                    {...register(`segments.${index}.constraints.${cIdx}.value`)}
                  />
                  {!locked ? (
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => constraintArray.remove(cIdx)}
                      aria-label="Remove constraint"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  ) : null}
                  {cErr?.property?.message || cErr?.value?.message ? (
                    <p className="col-span-full text-xs text-red-600">
                      {cErr?.property?.message ?? cErr?.value?.message}
                    </p>
                  ) : null}
                </div>
              );
            })}
          </div>
        )}
      </div>

      <div>
        <div className="mb-2 flex items-center justify-between">
          <span className="text-xs font-semibold uppercase tracking-wide text-slate-500">
            Distributions
          </span>
          <div className="flex items-center gap-2">
            <span
              className={cn(
                'inline-flex items-center gap-1 text-xs font-medium tabular-nums',
                sum === 100
                  ? 'text-emerald-600'
                  : sum > 100
                    ? 'text-red-600'
                    : 'text-amber-600',
              )}
            >
              {sum > 100 ? <AlertTriangle className="h-3.5 w-3.5" /> : null}
              Total: {sum}%
              {sum > 100 ? <span> · over by {overshoot}%</span> : null}
              {sum < 100 ? <span> · {undershoot}% unallocated</span> : null}
            </span>
            {!locked ? (
              <Button
                size="sm"
                variant="ghost"
                onClick={() =>
                  distArray.append({
                    variantKey: variantKeys[0] ?? '',
                    percent: 0,
                  })
                }
                disabled={variantKeys.length === 0}
              >
                <Plus className="h-4 w-4" />
                Add distribution
              </Button>
            ) : null}
          </div>
        </div>
        {err?.distributions?.message ? (
          <p className="mb-2 text-xs text-red-600">
            {err.distributions.message}
          </p>
        ) : null}
        {err?.distributions?.root?.message ? (
          <p className="mb-2 text-xs text-red-600">
            {err.distributions.root.message}
          </p>
        ) : null}
        <div className="flex flex-col gap-2">
          {distArray.fields.map((dField, dIdx) => {
            const dErr = err?.distributions?.[dIdx];
            return (
              <div
                key={dField.id}
                className="grid grid-cols-1 gap-2 rounded border border-slate-200 bg-white p-2 sm:grid-cols-[1fr_120px_auto]"
              >
                <Controller
                  control={control}
                  name={`segments.${index}.distributions.${dIdx}.variantKey`}
                  render={({ field }) => (
                    <Select
                      {...field}
                      disabled={locked}
                      aria-invalid={!!dErr?.variantKey}
                    >
                      <option value="">— pick variant —</option>
                      {variantKeys.map((k) => (
                        <option key={k} value={k}>
                          {k}
                        </option>
                      ))}
                    </Select>
                  )}
                />
                <Input
                  type="number"
                  min={0}
                  max={100}
                  disabled={locked}
                  aria-invalid={!!dErr?.percent}
                  {...register(
                    `segments.${index}.distributions.${dIdx}.percent`,
                    { valueAsNumber: true },
                  )}
                />
                {!locked && distArray.fields.length > 1 ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => distArray.remove(dIdx)}
                    aria-label="Remove distribution"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                ) : (
                  <span />
                )}
                {dErr?.variantKey?.message || dErr?.percent?.message ? (
                  <p className="col-span-full text-xs text-red-600">
                    {dErr?.variantKey?.message ?? dErr?.percent?.message}
                  </p>
                ) : null}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
