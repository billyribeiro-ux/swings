import { error } from '@sveltejs/kit';
import { courses } from '$lib/data/courses';
import type { PageLoad } from './$types';

export const load: PageLoad = ({ params }) => {
	const course = courses.find((c) => c.slug === params.slug);
	if (!course) error(404, 'Course not found');
	return { course };
};
