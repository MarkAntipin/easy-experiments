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

/**
 * Field-level edit policy:
 *   - `unlocked`: every field is editable (create + draft edit).
 *   - `rampUpOnly`: variants, key, primaryMetric, and segment shape are locked,
 *     but each segment's `rolloutPercent` and the description stay editable.
 *     Used while the experiment is running.
 *   - `fullyLocked`: only the description is editable. Used after stop.
 */
export type ExperimentFormLockMode = 'unlocked' | 'rampUpOnly' | 'fullyLocked';

export interface ExperimentFormProps {
  initial?: ExperimentDetail;
  mode: 'create' | 'edit';
  submitting: boolean;
  submitLabel: string;
  onSubmit: (payload: CreateExperimentRequest) => void | Promise<void>;
  lockMode?: ExperimentFormLockMode;
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
  lockMode = 'unlocked',
}: ExperimentFormProps) {
  const shapeLocked = lockMode !== 'unlocked';
  const rolloutLocked = lockMode === 'fullyLocked';

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
      {lockMode !== 'unlocked' ? <LockBanner mode={lockMode} /> : null}

      <section className="rounded-lg border border-slate-200 bg-white p-6">
        <h2 className="mb-1 text-base font-semibold text-slate-900">Basics</h2>
        <p className="mb-4 text-sm text-slate-500">
          A short identifier for this experiment and the metric you&rsquo;ll
          watch to decide which variant wins.
        </p>
        <div className="grid gap-4 sm:grid-cols-2">
          <Field
            label="Key"
            htmlFor="key"
            required
            readOnly={shapeLocked || mode === 'edit'}
            error={errors.key?.message}
            hint={
              shapeLocked || mode === 'edit'
                ? "Can't be changed after creation."
                : 'Unique identifier used at evaluation time.'
            }
          >
            <Input
              id="key"
              placeholder="homepage_cta"
              disabled={shapeLocked || mode === 'edit'}
              {...register('key')}
            />
          </Field>
          <Field
            label="Primary metric"
            htmlFor="primaryMetric"
            required
            error={errors.primaryMetric?.message}
            hint={
              shapeLocked
                ? "Can't be changed once the experiment has started."
                : "Identifier for the outcome you're optimizing — track it in your analytics so you can compare variants."
            }
          >
            <Input
              id="primaryMetric"
              placeholder="signup_conversion"
              disabled={shapeLocked}
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
        locked={shapeLocked}
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
        shapeLocked={shapeLocked}
        rolloutLocked={rolloutLocked}
      />

      <div className="flex items-center justify-end gap-2">
        <Button type="submit" loading={submitting}>
          {submitLabel}
        </Button>
      </div>
    </form>
  );
}

function LockBanner({ mode }: { mode: ExperimentFormLockMode }) {
  if (mode === 'rampUpOnly') {
    return (
      <div className="rounded-md border border-amber-200 bg-amber-50 p-4 text-base text-amber-900">
        <strong className="font-semibold">Running.</strong> You can edit the
        description and ramp up each segment&rsquo;s rollout %. Everything else
        is locked to keep the analysis comparable. Rollout can only go up,
        never down.
      </div>
    );
  }
  return (
    <div className="rounded-md border border-slate-200 bg-slate-50 p-4 text-base text-slate-700">
      <strong className="font-semibold">Stopped.</strong> Only the description
      is editable. To run a new test, create a new experiment.
    </div>
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
    <section className="rounded-lg border border-slate-200 bg-white p-6">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h2 className="flex items-center gap-2 text-base font-semibold text-slate-900">
            Variants
            {locked ? <ReadOnlyTag /> : null}
          </h2>
          <p className="text-sm text-slate-500">
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
            <Plus aria-hidden className="h-4 w-4" />
            Add variant
          </Button>
        ) : null}
      </div>

      {rootError ? (
        <p className="mb-3 text-sm text-red-600">{rootError}</p>
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
                <span className="text-base font-medium text-slate-700">Control</span>
                <label className="mt-1 inline-flex h-11 items-center gap-2">
                  <input
                    type="radio"
                    name="control-variant"
                    disabled={locked}
                    checked={liveIsControl}
                    onChange={() => onControlChange(idx)}
                    className="h-4 w-4 border-slate-300 text-brand-600 focus:ring-brand-500"
                  />
                  <span className="text-sm text-slate-500">Use as control</span>
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
              <div className="flex items-start pt-7">
                {!locked && array.fields.length > 1 ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => array.remove(idx)}
                    aria-label="Remove variant"
                  >
                    <Trash2 aria-hidden className="h-4 w-4" />
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
  shapeLocked,
  rolloutLocked,
}: {
  form: UseFormReturn<ExperimentFormValues>;
  array: ReturnType<typeof useFieldArray<ExperimentFormValues, 'segments'>>;
  variantKeys: string[];
  shapeLocked: boolean;
  rolloutLocked: boolean;
}) {
  const { formState: { errors } } = form;
  const rootError = errors.segments?.message ?? errors.segments?.root?.message;
  // Adding/removing segments is structural — locked whenever shape is locked.
  const canAddRemove = !shapeLocked;
  // Show a dedicated "Read-only" badge only when the segment is fully locked.
  // In rampUpOnly the rollout is editable, so a flat "Read-only" tag would
  // mislead.
  const showReadOnlyTag = shapeLocked && rolloutLocked;

  return (
    <section className="rounded-lg border border-slate-200 bg-white p-6">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h2 className="flex items-center gap-2 text-base font-semibold text-slate-900">
            Segments
            {showReadOnlyTag ? <ReadOnlyTag /> : null}
          </h2>
          <p className="text-sm text-slate-500">
            Decide who sees this experiment and how they&rsquo;re split. Lower
            priority numbers are checked first; the first matching segment
            wins.
          </p>
        </div>
        {canAddRemove ? (
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
            <Plus aria-hidden className="h-4 w-4" />
            Add segment
          </Button>
        ) : null}
      </div>

      {rootError ? (
        <p className="mb-3 text-sm text-red-600">{rootError}</p>
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
            shapeLocked={shapeLocked}
            rolloutLocked={rolloutLocked}
            onRemove={() => array.remove(idx)}
            canRemove={canAddRemove && array.fields.length > 1}
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
  shapeLocked,
  rolloutLocked,
  onRemove,
  canRemove,
}: {
  index: number;
  form: UseFormReturn<ExperimentFormValues>;
  variantKeys: string[];
  shapeLocked: boolean;
  rolloutLocked: boolean;
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
    <div className="rounded-md border border-slate-200 bg-slate-50/40 p-5">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-base font-medium text-slate-800">
          Segment #{index + 1}
        </h3>
        {canRemove ? (
          <Button
            size="sm"
            variant="ghost"
            onClick={onRemove}
            aria-label="Remove segment"
          >
            <Trash2 aria-hidden className="h-4 w-4" />
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
            disabled={shapeLocked}
            {...register(`segments.${index}.priority`, { valueAsNumber: true })}
          />
        </Field>
        <Field
          label="Rollout %"
          required
          error={err?.rolloutPercent?.message}
          hint={
            shapeLocked && !rolloutLocked
              ? 'Adjustable while running — can only increase, not decrease.'
              : 'Portion of eligible users bucketed into this segment.'
          }
        >
          <Input
            type="number"
            min={0}
            max={100}
            disabled={rolloutLocked}
            {...register(`segments.${index}.rolloutPercent`, {
              valueAsNumber: true,
            })}
          />
        </Field>
      </div>

      <div className="mb-4">
        <div className="mb-2 flex items-center justify-between">
          <div>
            <span className="text-sm font-semibold uppercase tracking-wide text-slate-500">
              Constraints
            </span>
            <p className="mt-0.5 text-sm text-slate-500">
              Filter who matches this segment. Property names come from the{' '}
              <code className="rounded bg-slate-100 px-1 font-mono">
                properties
              </code>{' '}
              object you pass to evaluate (e.g. country, plan_tier, deviceType).
            </p>
          </div>
          {!shapeLocked ? (
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
              <Plus aria-hidden className="h-4 w-4" />
              Add constraint
            </Button>
          ) : null}
        </div>
        {constraintArray.fields.length === 0 ? (
          <p className="text-sm text-slate-500">
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
                    disabled={shapeLocked}
                    aria-invalid={!!cErr?.property}
                    {...register(`segments.${index}.constraints.${cIdx}.property`)}
                  />
                  <Controller
                    control={control}
                    name={`segments.${index}.constraints.${cIdx}.operator`}
                    render={({ field }) => (
                      <Select {...field} disabled={shapeLocked}>
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
                    disabled={shapeLocked}
                    aria-invalid={!!cErr?.value}
                    {...register(`segments.${index}.constraints.${cIdx}.value`)}
                  />
                  {!shapeLocked ? (
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => constraintArray.remove(cIdx)}
                      aria-label="Remove constraint"
                    >
                      <Trash2 aria-hidden className="h-4 w-4" />
                    </Button>
                  ) : null}
                  {cErr?.property?.message || cErr?.value?.message ? (
                    <p className="col-span-full text-sm text-red-600">
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
          <span className="text-sm font-semibold uppercase tracking-wide text-slate-500">
            Distributions
          </span>
          <div className="flex items-center gap-2">
            <span
              className={cn(
                'inline-flex items-center gap-1 text-sm font-medium tabular-nums',
                sum === 100
                  ? 'text-emerald-600'
                  : sum > 100
                    ? 'text-red-600'
                    : 'text-amber-600',
              )}
            >
              {sum > 100 ? <AlertTriangle aria-hidden className="h-4 w-4" /> : null}
              Total: {sum}%
              {sum > 100 ? <span> · over by {overshoot}%</span> : null}
              {sum < 100 ? <span> · {undershoot}% unallocated</span> : null}
            </span>
            {!shapeLocked ? (
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
                <Plus aria-hidden className="h-4 w-4" />
                Add distribution
              </Button>
            ) : null}
          </div>
        </div>
        {err?.distributions?.message ? (
          <p className="mb-2 text-sm text-red-600">
            {err.distributions.message}
          </p>
        ) : null}
        {err?.distributions?.root?.message ? (
          <p className="mb-2 text-sm text-red-600">
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
                      disabled={shapeLocked}
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
                  disabled={shapeLocked}
                  aria-invalid={!!dErr?.percent}
                  {...register(
                    `segments.${index}.distributions.${dIdx}.percent`,
                    { valueAsNumber: true },
                  )}
                />
                {!shapeLocked && distArray.fields.length > 1 ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => distArray.remove(dIdx)}
                    aria-label="Remove distribution"
                  >
                    <Trash2 aria-hidden className="h-4 w-4" />
                  </Button>
                ) : (
                  <span />
                )}
                {dErr?.variantKey?.message || dErr?.percent?.message ? (
                  <p className="col-span-full text-sm text-red-600">
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

function ReadOnlyTag() {
  return (
    <span className="rounded bg-ink-100 px-1.5 py-0.5 text-xs font-medium uppercase tracking-wide text-ink-500">
      Read-only
    </span>
  );
}
