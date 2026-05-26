# pipeline
# on s3:///pipelines/km2/raw_events.csv
# retry 3
# Bash sink: archives the raw seed file. Bash has no body-level asset
# inference, so EVERY asset reference is via the annotation header — the
# perfect surface to sanity-check the annotation-only form.

echo "Archive run at $(date -Iseconds)"
echo "Source: s3://pipelines/km2/raw_events.csv"
echo "(In a real pipeline this would shell out to the wmill CLI to copy"
echo " the source object into a cold-storage bucket. No-op for sanity)"
