import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request }) => {
	try {
		const { email } = await request.json();

		if (!email || !email.includes('@')) {
			return json({ error: 'Valid email is required' }, { status: 400 });
		}

		// TODO: Integrate with email service (e.g., SendGrid, Mailgun, Resend)
		// For now, we'll just log and return success
		console.log(`Greeks PDF requested by: ${email}`);

		// In production, you would:
		// 1. Add email to your mailing list (e.g., ConvertKit, Mailchimp)
		// 2. Send email with PDF attachment or download link
		// 3. Track conversion in analytics

		// Example with a hypothetical email service:
		// await emailService.send({
		//   to: email,
		//   subject: 'Your Free Options Greeks Guide',
		//   template: 'greeks-pdf',
		//   attachments: [{ filename: 'options-greeks-guide.pdf', path: '/pdfs/greeks.pdf' }]
		// });

		return json({
			success: true,
			message: 'PDF sent successfully'
		});
	} catch (error) {
		console.error('Greeks PDF request error:', error);
		return json({ error: 'Failed to process request. Please try again.' }, { status: 500 });
	}
};
